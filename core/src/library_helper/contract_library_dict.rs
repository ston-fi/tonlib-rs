use super::{LibraryHelper, TonLibraryError};
use crate::cell::BagOfCells;
use crate::TonHash;

#[derive(Debug, PartialEq)]
pub struct ContractLibraryDict(pub Vec<u8>);

impl ContractLibraryDict {
    pub fn new(dict_boc: Vec<u8>) -> Self {
        Self(dict_boc)
    }

    pub fn keys(&self) -> Result<Vec<TonHash>, TonLibraryError> {
        let cells = BagOfCells::parse(&self.0)?.single_root()?;
        LibraryHelper::extract_library_hashes(&[cells])
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;

    use crate::library_helper::ContractLibraryDict;
    use crate::TonHash;

    #[test]
    fn test_contract_library_dict_keys() -> anyhow::Result<()> {
        let dict_boc = BASE64_STANDARD.decode("te6ccgECDwEAAwIAA7V7nBIxv16jVmxIK1r2UBfaInVF/ZiTIhIyZW89zjsyXFAAApNHjvgAGEQXStaTphNowXid2UrUP36WDrzwLiBnIAdPV/U5Nn4AAAKTQX9XNIZfX1UAADR1FsgIBQQBAhkEgF4JAsnmyhh0c9cRAwIAb8mHoSBMFFhAAAAAAAACAAAAAAADGN3lQzL4zq6hI/88sLWZIBMzC7WNYeJBkUVxatc74GBAUBcMAJ5FPGwLbCQAAAAAAAAAAScAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIJyZKZhyT+7yUFKUgAlQcP6fYA6vuPZJ3iavbRwpvRXkaLpxCmiu6kCFcIf9iDkMi+QzAWWce3or9ccVakMUVmEJwIB4AgGAQHfBwC5aAFzgkY369RqzYkFa17KAvtETqi/sxJkQkZMree5x2ZLiwAlyKt1kViW7ZNClbNx3193SCmWemxLikIPCmsjqS7+uVAp0ujABhRYYAAAUmjx3wAEy+vqoB1KHnNAArFoAeU6L0QTxXmMpSLbAFdNPEeWbNwvX1rD8Ipmh3N0p1nJAC5wSMb9eo1ZsSCta9lAX2iJ1Rf2YkyISMmVvPc47MlxUCyebKAGRvwEAABSaPBSRYTL6+pB4AwJAl9uPE8JwA7MY+ZmnGSzCzr1ac6euD6OttvkvYjYIUAO1kagpLqFujmEP0MUdJPyDIILCgApAAAAAb8I6wBl9fYJAwCuM0M2BgBACEIC8F5zC6xlKwQUtGc2RJmcgbi9KFlYBMAU/fgHgoJ5lykCATQODQDLgBLkVbrIrEt2yaFK2bjvr7ukFMs9NiXFIQeFNZHUl39csAPKdF6IJ4rzGUpFtgCumniPLNm4Xr61h+EUzQ7m6U6zkgBbNNH1WCqtrdoD1I2iZsPakLqhenpsHNvLIVoFAC8ac4f8CEICcmEyBp1jWPl1TYnsWJ5gUk5EnB0PEQTb4dSXIG+LspA=")?;
        let dict = ContractLibraryDict::new(dict_boc);
        let expected_hashes = vec![
            TonHash::from_hex("726132069d6358f9754d89ec589e60524e449c1d0f1104dbe1d497206f8bb290")?,
            TonHash::from_hex("f05e730bac652b0414b4673644999c81b8bd28595804c014fdf8078282799729")?,
        ];
        let keys = dict.keys()?;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&expected_hashes[0]));
        assert!(keys.contains(&expected_hashes[1]));
        Ok(())
    }
}
