use std::sync::Arc;

use tonlib_core::TonAddress;

use super::{ContractLibraryDict, LibraryLoader};
use crate::contract::TonContractError;
use crate::tl::RawFullAccountState;

#[derive(Clone)]
pub struct LibraryProvider {
    loader: Arc<dyn LibraryLoader>,
}

impl LibraryProvider {
    pub fn new(loader: Arc<dyn LibraryLoader>) -> LibraryProvider {
        LibraryProvider { loader }
    }

    pub async fn get_contract_libraries(
        &self,
        address: &TonAddress,
        account_state: &Arc<RawFullAccountState>,
    ) -> Result<Arc<ContractLibraryDict>, TonContractError> {
        self.get_libraries_by_contract_code(address, &account_state.code)
            .await
    }

    pub async fn get_libraries_by_contract_code(
        &self,
        address: &TonAddress,
        code: &[u8],
    ) -> Result<Arc<ContractLibraryDict>, TonContractError> {
        // TODO cache
        self.loader.load_contract_libraries(address, code).await
    }
}
