use std::collections::{HashMap, HashSet, VecDeque};

use tonlib_core::cell::dict::predefined_writers::val_writer_ref_cell;
use tonlib_core::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib_core::TonHash;

use super::ContractLibraryDict;
use crate::contract::TonLibraryError;

pub struct LibraryHelper;

impl LibraryHelper {
    pub fn store_to_dict(
        lib_hashmap: HashMap<TonHash, ArcCell>,
    ) -> Result<ContractLibraryDict, TonLibraryError> {
        if lib_hashmap.is_empty() {
            return Ok(ContractLibraryDict {
                dict_boc: Vec::new(),
                keys: Vec::new(),
            });
        }

        let keys = lib_hashmap.keys().cloned().collect();
        let lib_cell = CellBuilder::new()
            .store_dict_data(256, val_writer_ref_cell, lib_hashmap)?
            .build()?;

        let dict_boc = BagOfCells::from_root(lib_cell).serialize(false)?;
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
    use std::collections::HashSet;

    use tokio_test::assert_ok;
    use tonlib_core::cell::BagOfCells;
    use tonlib_core::TonHash;

    use crate::contract::LibraryHelper;

    #[test]
    fn test_get_lib_hashes_by_code() -> anyhow::Result<()> {
        let boc = hex::decode("b5ee9c72410101010023000842029f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a866a5f879")?;
        let expected_lib_id =
            TonHash::from_hex("9f31f4f413a3accb706c88962ac69d59103b013a0addcfaeed5dd73c18fa98a8")?;
        let code = BagOfCells::parse(&boc)?.single_root()?;
        let hashes = assert_ok!(LibraryHelper::extract_library_hashes(&[code]));

        assert_eq!(hashes.len(), 1);
        assert_eq!(expected_lib_id, hashes[0]);

        Ok(())
    }

    #[test]
    fn test_extract_libs_hashes_from_account() -> anyhow::Result<()> {
        let account_cell = BagOfCells::parse_base64("te6ccgEBBAEA8wACbcALnBIxv16jVmxIK1r2UBfaInVF/ZiTIhIyZW89zjsyXFIIgXjDL67PEAAApNBf1c0o5iWgE0ADAQHLgBLkVbrIrEt2yaFK2bjvr7ukFMs9NiXFIQeFNZHUl39csAPKdF6IJ4rzGUpFtgCumniPLNm4Xr61h+EUzQ7m6U6zkgBbNNH1WCqtrdoD1I2iZsPakLqhenpsHNvLIVoFAC8ac5f8AgBTgB2Yx8zNOMlmFnXq0509cH0dbbfJexGwQoAdrI1BSXULYF9eEAC+vCAQCEIC8F5zC6xlKwQUtGc2RJmcgbi9KFlYBMAU/fgHgoJ5lyk=")?.single_root()?;

        let expected_hash =
            TonHash::from_hex("f05e730bac652b0414b4673644999c81b8bd28595804c014fdf8078282799729")?;
        let hashes = LibraryHelper::extract_library_hashes(&[account_cell])?;
        assert_eq!([expected_hash], hashes.as_slice());
        Ok(())
    }

    #[test]
    fn test_extract_libs_hashes_from_tx() -> anyhow::Result<()> {
        let tx_cell = BagOfCells::parse_base64("te6ccgECDwEAAwIAA7V7nBIxv16jVmxIK1r2UBfaInVF/ZiTIhIyZW89zjsyXFAAApNHjvgAGEQXStaTphNowXid2UrUP36WDrzwLiBnIAdPV/U5Nn4AAAKTQX9XNIZfX1UAADR1FsgIBQQBAhkEgF4JAsnmyhh0c9cRAwIAb8mHoSBMFFhAAAAAAAACAAAAAAADGN3lQzL4zq6hI/88sLWZIBMzC7WNYeJBkUVxatc74GBAUBcMAJ5FPGwLbCQAAAAAAAAAAScAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIJyZKZhyT+7yUFKUgAlQcP6fYA6vuPZJ3iavbRwpvRXkaLpxCmiu6kCFcIf9iDkMi+QzAWWce3or9ccVakMUVmEJwIB4AgGAQHfBwC5aAFzgkY369RqzYkFa17KAvtETqi/sxJkQkZMree5x2ZLiwAlyKt1kViW7ZNClbNx3193SCmWemxLikIPCmsjqS7+uVAp0ujABhRYYAAAUmjx3wAEy+vqoB1KHnNAArFoAeU6L0QTxXmMpSLbAFdNPEeWbNwvX1rD8Ipmh3N0p1nJAC5wSMb9eo1ZsSCta9lAX2iJ1Rf2YkyISMmVvPc47MlxUCyebKAGRvwEAABSaPBSRYTL6+pB4AwJAl9uPE8JwA7MY+ZmnGSzCzr1ac6euD6OttvkvYjYIUAO1kagpLqFujmEP0MUdJPyDIILCgApAAAAAb8I6wBl9fYJAwCuM0M2BgBACEIC8F5zC6xlKwQUtGc2RJmcgbi9KFlYBMAU/fgHgoJ5lykCATQODQDLgBLkVbrIrEt2yaFK2bjvr7ukFMs9NiXFIQeFNZHUl39csAPKdF6IJ4rzGUpFtgCumniPLNm4Xr61h+EUzQ7m6U6zkgBbNNH1WCqtrdoD1I2iZsPakLqhenpsHNvLIVoFAC8ac4f8CEICcmEyBp1jWPl1TYnsWJ5gUk5EnB0PEQTb4dSXIG+LspA=")?.single_root()?;

        let expected_hashes = HashSet::from([
            TonHash::from_hex("f05e730bac652b0414b4673644999c81b8bd28595804c014fdf8078282799729")?,
            TonHash::from_hex("726132069d6358f9754d89ec589e60524e449c1d0f1104dbe1d497206f8bb290")?,
        ]);
        let hashes = HashSet::from_iter(LibraryHelper::extract_library_hashes(&[tx_cell])?);
        assert_eq!(hashes, expected_hashes);

        Ok(())
    }
}
