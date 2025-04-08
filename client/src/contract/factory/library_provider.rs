use async_trait::async_trait;
use tonlib_core::cell::ArcCell;
use tonlib_core::library_helper::{ContractLibraryDict, TonLibraryError};

#[async_trait]
pub trait LibraryProvider: Send + Sync {
    async fn get_libs(
        &self,
        cells: &[ArcCell],
        mc_seqno: Option<i32>,
    ) -> Result<ContractLibraryDict, TonLibraryError>;
}
