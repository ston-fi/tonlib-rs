use async_trait::async_trait;
use tonlib_core::cell::ArcCell;
use tonlib_core::TonHash;

use crate::contract::TonLibraryError;

#[derive(Debug, PartialEq)]
pub struct ContractLibraryDict {
    pub dict_boc: Vec<u8>,
    pub keys: Vec<TonHash>,
}

#[async_trait]
pub trait LibraryProvider: Send + Sync {
    async fn get_libs(
        &self,
        cells: &[ArcCell],
        mc_seqno: Option<i32>,
    ) -> Result<ContractLibraryDict, TonLibraryError>;
}
