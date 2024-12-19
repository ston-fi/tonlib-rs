use std::sync::Arc;

use async_trait::async_trait;
use tonlib_core::cell::{ArcCell, BagOfCells};
use tonlib_core::TonHash;

use crate::client::{TonClient, TonClientInterface};
use crate::contract::TonLibraryError;
use crate::tl::TonLibraryId;

#[derive(Debug)]
pub struct ContractLibraryDict {
    pub dict_boc: Vec<u8>,
    pub keys: Vec<TonLibraryId>,
}

#[async_trait]
pub trait LibraryLoader: Send + Sync {
    async fn get_library(&self, hash: &TonHash) -> Result<Option<ArcCell>, TonLibraryError>;

    async fn get_libraries(&self, hashes: &[TonHash]) -> Result<Vec<ArcCell>, TonLibraryError>;
}

pub struct DefaultLibraryLoader {
    client: TonClient,
}

impl DefaultLibraryLoader {
    pub fn new(client: &TonClient) -> Arc<Self> {
        Arc::new(DefaultLibraryLoader {
            client: client.clone(),
        })
    }
}

#[async_trait]
impl LibraryLoader for DefaultLibraryLoader {
    async fn get_library(&self, hash: &TonHash) -> Result<Option<ArcCell>, TonLibraryError> {
        let library_result = self.get_libraries(&[*hash]).await?;
        match library_result.len() {
            0 => {
                log::warn!("Library not found for {:?}", hash);
                Ok(None)
            }
            1 => Ok(Some(library_result[0].clone())),
            _ => Err(TonLibraryError::MultipleLibrariesReturned),
        }
    }

    async fn get_libraries(&self, hashes: &[TonHash]) -> Result<Vec<ArcCell>, TonLibraryError> {
        let mut results = Vec::new();

        // If hashes exceed MAX_LIBS_REQUESTED, split them into chunks
        for chunk in hashes.chunks(Self::MAX_LIBS_REQUESTED) {
            let mut partial_result = self.get_libraries_impl(chunk).await?;
            results.append(&mut partial_result);
        }
        Ok(results)
    }
}

impl DefaultLibraryLoader {
    const MAX_LIBS_REQUESTED: usize = 255;

    async fn get_libraries_impl(
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
