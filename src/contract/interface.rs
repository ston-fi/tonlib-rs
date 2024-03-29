use async_trait::async_trait;

use super::TonContractError;
use crate::address::TonAddress;
use crate::client::TonClientInterface;
use crate::tl::{SmcRunResult, TvmCell, TvmStackEntry};
use crate::types::TonMethodId;

#[async_trait]
pub trait TonContractInterface {
    fn client(&self) -> &dyn TonClientInterface;

    fn address(&self) -> &TonAddress;

    async fn get_code_cell(&self) -> Result<TvmCell, TonContractError>;

    async fn get_data_cell(&self) -> Result<TvmCell, TonContractError>;

    async fn get_state_cell(&self) -> Result<TvmCell, TonContractError>;

    #[allow(clippy::ptr_arg)]
    async fn run_get_method<A: Into<TonMethodId> + Send>(
        &self,
        method: A,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError>;
}
