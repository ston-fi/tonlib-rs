use std::sync::Arc;

use async_trait::async_trait;
use tonlib_core::cell::{ArcCell, BagOfCells};
use tonlib_core::TonHash;

use crate::client::{TonClient, TonClientInterface};
use crate::contract::TonLibraryError;
use crate::tl::TonLibraryId;

#[derive(Debug, PartialEq)]
pub struct ContractLibraryDict {
    pub dict_boc: Vec<u8>,
    pub keys: Vec<TonHash>,
}

#[async_trait]
pub trait LibraryLoader: Send + Sync {
    async fn load_libraries(
        &self,
        hashes: &[TonHash],
        seqno: Option<i32>,
    ) -> Result<Vec<ArcCell>, TonLibraryError>;
}

pub struct BlockchainLibraryLoader {
    client: TonClient,
}

impl BlockchainLibraryLoader {
    pub fn new(client: &TonClient) -> Arc<Self> {
        Arc::new(BlockchainLibraryLoader {
            client: client.clone(),
        })
    }
}

#[async_trait]
impl LibraryLoader for BlockchainLibraryLoader {
    async fn load_libraries(
        &self,
        hashes: &[TonHash],
        _seqno: Option<i32>,
    ) -> Result<Vec<ArcCell>, TonLibraryError> {
        let mut results = Vec::new();

        // If hashes exceed MAX_LIBS_REQUESTED, split them into chunks
        for chunk in hashes.chunks(Self::MAX_LIBS_REQUESTED) {
            let mut partial_result = self.load_libraries_impl(chunk).await?;
            results.append(&mut partial_result);
        }
        Ok(results)
    }
}

impl BlockchainLibraryLoader {
    const MAX_LIBS_REQUESTED: usize = 255;

    async fn load_libraries_impl(
        &self,
        hashes: &[TonHash],
    ) -> Result<Vec<ArcCell>, TonLibraryError> {
        let library_list: Vec<_> = hashes
            .iter()
            .map(|hash| TonLibraryId::from(*hash))
            .collect();
        let library_result = self.client.smc_get_libraries(&library_list).await?;

        let libraries: Vec<ArcCell> = library_result
            .result
            .into_iter()
            .map(|lib| BagOfCells::parse(&lib.data)?.into_single_root())
            .collect::<Result<_, _>>()?;

        Ok(libraries)
    }
}
