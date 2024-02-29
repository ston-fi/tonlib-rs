use std::sync::Arc;

use async_trait::async_trait;
use lazy_static::lazy_static;

use crate::contract::TonContractError;

#[derive(Clone)]
pub struct LibraryProvider {
    loader: Arc<dyn LibraryLoader>,
}

impl LibraryProvider {
    pub async fn get_library(&self, hash: &str) -> Result<Option<String>, TonContractError> {
        self.loader.load_library(hash).await
    }
}

#[async_trait]
pub trait LibraryLoader: Send + Sync {
    // TODO: Param type might be &[u8] or &[u8; 32] or &str
    // TODO: Result type might be String, Vec<u8> or Cell or BagOfCells
    async fn load_library(&self, hash: &str) -> Result<Option<String>, TonContractError>;
}

lazy_static! {
    pub static ref DUMMY_LIBRARY_PROVIDER: LibraryProvider = LibraryProvider {
        loader: Arc::new(DummyLibraryLoader {})
    };
}

pub struct DummyLibraryLoader {}

#[async_trait]
impl LibraryLoader for DummyLibraryLoader {
    async fn load_library(&self, _hash: &str) -> Result<Option<String>, TonContractError> {
        todo!()
    }
}
