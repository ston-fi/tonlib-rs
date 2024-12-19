use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use tonlib_core::cell::dict::predefined_writers::val_writer_ref_cell;
use tonlib_core::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib_core::TonHash;

use super::{ContractLibraryDict, LibraryLoader};
use crate::contract::TonLibraryError;

#[derive(Clone)]
pub struct LibraryProvider {
    loader: Arc<dyn LibraryLoader>,
}

impl LibraryProvider {
    pub fn new(loader: Arc<dyn LibraryLoader>) -> Self {
        Self { loader }
    }

    pub fn extract_library_hashes<'a, I>(cells: I) -> Result<Vec<TonHash>, TonLibraryError>
    where
        I: IntoIterator<Item = &'a ArcCell>,
    {
        let mut arc_cell_buffer = VecDeque::new();

        arc_cell_buffer.extend(cells);

        let mut lib_cells = HashSet::new();
        let mut visited_cells = HashSet::new();

        while let Some(cell) = arc_cell_buffer.pop_front() {
            if !visited_cells.insert(cell) {
                continue;
            }
            let refs = cell.references();
            arc_cell_buffer.extend(refs.iter());
            if cell.is_library() {
                lib_cells.insert(cell.clone());
            }
        }

        let lib_hashes: Vec<TonHash> = lib_cells
            .iter()
            .map(|i| i.data()[1..].try_into())
            .collect::<Result<_, _>>()?;

        Ok(lib_hashes)
    }

    pub async fn get_libs_dict<'a, I>(
        &self,
        cells: I,
        seqno: Option<i32>,
    ) -> Result<ContractLibraryDict, TonLibraryError>
    where
        I: IntoIterator<Item = &'a ArcCell>,
    {
        let refs = LibraryProvider::extract_library_hashes(cells)?;

        let libs = self.loader.load_libraries(refs.as_slice(), seqno).await?;

        let lib_hashmap = libs.into_iter().map(|l| (l.cell_hash(), l)).collect();

        let lib_cell = CellBuilder::new()
            .store_dict_data(256, val_writer_ref_cell, lib_hashmap)?
            .build()?;

        let dict_boc = BagOfCells::from_root(lib_cell).serialize(false)?;

        let keys = refs.iter().map(|r| (*r).into()).collect();

        let dict = ContractLibraryDict { dict_boc, keys };
        Ok(dict)
    }
}
