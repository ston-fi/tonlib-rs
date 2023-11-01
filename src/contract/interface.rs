use async_trait::async_trait;

use crate::address::TonAddress;
use crate::client::TonClientInterface;
use crate::tl::{SmcRunResult, TvmCell, TvmStackEntry};

use super::TonContractError;

#[async_trait]
pub trait TonContractInterface {
    fn client(&self) -> &dyn TonClientInterface;

    fn address(&self) -> &TonAddress;

    async fn get_code(&self) -> Result<TvmCell, TonContractError>;

    async fn get_data(&self) -> Result<TvmCell, TonContractError>;

    async fn get_state(&self) -> Result<TvmCell, TonContractError>;

    async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError>;
}
