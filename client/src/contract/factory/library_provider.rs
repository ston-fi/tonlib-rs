use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use tonlib_core::cell::ArcCell;
use tonlib_core::library_helper::{ContractLibraryDict, TonLibraryError};
use tonlib_core::TonHash;

#[async_trait]
pub trait LibraryProvider: Send + Sync {
    async fn get_or_load_libs(
        &self,
        lib_ids: HashSet<TonHash>,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError>;

    async fn get_or_load_code_libs(
        &self,
        code: TonHash,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError>;
    fn update_code_libs(&self, code: TonHash, lib_id: TonHash);
}
