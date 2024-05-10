use std::sync::Arc;

use super::{ContractLibraryDict, LibraryLoader};
use crate::address::TonAddress;
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
        let code = &account_state.code;

        //todo cache

        self.loader.load_contract_libraries(address, code).await
    }
}
