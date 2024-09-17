use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::cell::{ArcCell, BagOfCells, Cell, RawBagOfCells, RawCell, TonCellError};
use crate::TonHash;

#[derive(Debug, Clone)]
struct IndexedCell {
    index: usize,
    cell: ArcCell,
}

pub(crate) fn convert_to_raw_boc(boc: &BagOfCells) -> Result<RawBagOfCells, TonCellError> {
    let cells_by_hash = build_and_verify_index(&boc.roots);

    // Sort indexed cells by their index value.
    let mut index_slice: Vec<_> = cells_by_hash.values().collect();
    index_slice.sort_unstable_by(|a, b| a.borrow().index.cmp(&b.borrow().index));

    // Remove gaps in indices.
    index_slice
        .iter()
        .enumerate()
        .for_each(|(real_index, indexed_cell)| indexed_cell.borrow_mut().index = real_index);

    let cells_iter = index_slice
        .into_iter()
        .map(|indexed_cell| indexed_cell.borrow().cell.clone());
    let raw_cells = raw_cells_from_cells(cells_iter, &cells_by_hash)?;
    let root_indices = root_indices(&boc.roots, &cells_by_hash)?;

    Ok(RawBagOfCells {
        cells: raw_cells,
        roots: root_indices,
    })
}

fn build_and_verify_index(roots: &[ArcCell]) -> HashMap<TonHash, RefCell<IndexedCell>> {
    let mut current_cells: Vec<_> = roots.iter().map(Arc::clone).collect();
    let mut new_hash_index = 0;
    let mut cells_by_hash = HashMap::new();

    // Process cells to build the initial index.
    while !current_cells.is_empty() {
        let mut next_cells = Vec::with_capacity(current_cells.len() * 4);
        for cell in current_cells.iter() {
            let hash = cell.cell_hash();

            if cells_by_hash.contains_key(&hash) {
                continue; // Skip if already indexed.
            }

            cells_by_hash.insert(
                hash,
                RefCell::new(IndexedCell {
                    cell: Arc::clone(cell),
                    index: new_hash_index,
                }),
            );

            new_hash_index += 1;
            next_cells.extend(cell.references.clone()); // Add referenced cells for the next iteration.
        }

        current_cells = next_cells;
    }

    // Ensure indices are in the correct order based on cell references.
    let mut verify_order = true;
    while verify_order {
        verify_order = false;

        for index_cell in cells_by_hash.values() {
            for reference in index_cell.borrow().cell.references.iter() {
                let ref_hash = reference.cell_hash();
                if let Some(id_ref) = cells_by_hash.get(&ref_hash) {
                    if id_ref.borrow().index < index_cell.borrow().index {
                        id_ref.borrow_mut().index = new_hash_index;
                        new_hash_index += 1;
                        verify_order = true; // Reverify if an index was updated.
                    }
                }
            }
        }
    }

    cells_by_hash
}

fn root_indices(
    roots: &[ArcCell],
    cells_dict: &HashMap<TonHash, RefCell<IndexedCell>>,
) -> Result<Vec<usize>, TonCellError> {
    roots
        .iter()
        .map(|root_cell| root_cell.cell_hash())
        .map(|root_cell_hash| {
            cells_dict
                .get(&root_cell_hash)
                .map(|index_record| index_record.borrow().index)
                .ok_or_else(|| {
                    TonCellError::BagOfCellsSerializationError(format!(
                        "Couldn't find cell with hash {root_cell_hash:?} while searching for roots"
                    ))
                })
        })
        .collect()
}

fn raw_cells_from_cells(
    cells: impl Iterator<Item = ArcCell>,
    cells_by_hash: &HashMap<TonHash, RefCell<IndexedCell>>,
) -> Result<Vec<RawCell>, TonCellError> {
    cells
        .map(|cell| raw_cell_from_cell(&cell, cells_by_hash))
        .collect()
}

fn raw_cell_from_cell(
    cell: &Cell,
    cells_by_hash: &HashMap<TonHash, RefCell<IndexedCell>>,
) -> Result<RawCell, TonCellError> {
    raw_cell_reference_indices(cell, cells_by_hash).map(|reference_indices| {
        RawCell::new(
            cell.data.clone(),
            cell.bit_len,
            reference_indices,
            cell.get_level_mask(),
            cell.is_exotic(),
        )
    })
}

fn raw_cell_reference_indices(
    cell: &Cell,
    cells_by_hash: &HashMap<TonHash, RefCell<IndexedCell>>,
) -> Result<Vec<usize>, TonCellError> {
    cell.references
        .iter()
        .map(|cell| {
            cells_by_hash
                .get(&cell.cell_hash())
                .ok_or_else(|| {
                    TonCellError::BagOfCellsSerializationError(format!(
                        "Couldn't find cell with hash {:?} while searching for references",
                        cell.cell_hash()
                    ))
                })
                .map(|cell| cell.borrow().index)
        })
        .collect()
}
