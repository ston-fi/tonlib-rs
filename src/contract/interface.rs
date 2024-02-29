use std::sync::Arc;

use async_trait::async_trait;

use super::TonContractError;
use crate::address::TonAddress;
use crate::client::TonConnection;
use crate::contract::TonContractFactory;
use crate::tl::RawFullAccountState;
use crate::types::{TonMethodId, TvmStackEntry, TvmSuccess};

pub struct LoadedSmcState {
    pub conn: TonConnection,
    pub id: i64,
}

#[async_trait]
pub trait TonContractInterface {
    fn factory(&self) -> &TonContractFactory;

    fn address(&self) -> &TonAddress;

    async fn get_account_state(&self) -> Result<Arc<RawFullAccountState>, TonContractError>;

    async fn run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send;
}
