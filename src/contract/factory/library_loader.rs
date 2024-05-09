use std::sync::Arc;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::address::TonAddress;
use crate::client::{TonClient, TonClientInterface};
use crate::contract::TonContractError;
use crate::tl::{SmcLibraryQueryExt, TonLibraryId};

pub struct ContractLibraryDict {
    pub dict_boc: Vec<u8>,
    pub keys: Vec<TonLibraryId>,
}

#[async_trait]
pub trait LibraryLoader: Send + Sync {
    async fn load_contract_libraries(
        &self,
        address: &TonAddress,
        code: &[u8],
    ) -> Result<Arc<ContractLibraryDict>, TonContractError>;
}

pub struct DefaultLibraryLoader {
    client: TonClient,
}

impl DefaultLibraryLoader {
    pub fn new(client: &TonClient) -> Self {
        DefaultLibraryLoader {
            client: client.clone(),
        }
    }
}

#[async_trait]
impl LibraryLoader for DefaultLibraryLoader {
    async fn load_contract_libraries(
        &self,
        address: &TonAddress,
        code: &[u8],
    ) -> Result<Arc<ContractLibraryDict>, TonContractError> {
        const DEFAULT_MAX_LIBS: i32 = 255;
        let library_query = SmcLibraryQueryExt::ScanBoc {
            boc: code.to_vec(),
            max_libs: DEFAULT_MAX_LIBS,
        };
        let library_result = self.client.smc_get_libraries_ext(&[library_query]).await?;
        if !library_result.libs_not_found.is_empty() {
            let missing_libs = library_result
                .libs_not_found
                .iter()
                .map(|l| STANDARD.encode(&l.id))
                .collect();
            return Err(TonContractError::LibraryNotFound {
                address: address.clone(),
                missing_library: missing_libs,
            });
        }

        let dict_boc = library_result.dict_boc;
        let keys = library_result.libs_ok;

        let contract_libraies = ContractLibraryDict { dict_boc, keys };
        Ok(Arc::new(contract_libraies))
    }
}
