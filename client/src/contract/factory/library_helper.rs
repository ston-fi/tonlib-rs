use std::collections::{HashMap, HashSet, VecDeque};

use tonlib_core::cell::dict::predefined_writers::val_writer_ref_cell;
use tonlib_core::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib_core::TonHash;

use super::ContractLibraryDict;
use crate::contract::TonLibraryError;

pub struct LibraryHelper;

impl LibraryHelper {
    pub fn store_to_dict(
        library_hashes: &[TonHash],
        lib_hashmap: HashMap<TonHash, ArcCell>,
    ) -> Result<ContractLibraryDict, TonLibraryError> {
        let lib_cell = CellBuilder::new()
            .store_dict_data(256, val_writer_ref_cell, lib_hashmap)?
            .build()?;

        let dict_boc = BagOfCells::from_root(lib_cell).serialize(false)?;
        let keys = library_hashes.to_vec();
        let dict = ContractLibraryDict { dict_boc, keys };
        Ok(dict)
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
}

#[cfg(test)]
mod tests {

    use tokio_test::assert_ok;
    use tonlib_core::cell::BagOfCells;
    use tonlib_core::TonHash;

    use crate::contract::LibraryHelper;

    #[test]
    fn test_get_lib_hashes_by_code() -> anyhow::Result<()> {
        let boc =hex::decode("b5ee9c72410101010023000842029f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a866a5f879").unwrap();
        let expected_lib_id =
            TonHash::from_hex("9f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a8")
                .unwrap();
        let code = BagOfCells::parse(&boc)?.into_single_root()?;
        let hashes = assert_ok!(LibraryHelper::extract_library_hashes(&[code]));

        assert_eq!(hashes.len(), 1);
        assert_eq!(expected_lib_id, hashes[0]);

        Ok(())
    }
}
