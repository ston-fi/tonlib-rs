use async_trait::async_trait;

pub use error::*;
pub use factory::*;
pub use interface::*;
pub use jetton::*;
pub use latest_transactions_cache::*;
pub use nft::*;
pub use state::*;
pub use wallet::*;

use crate::address::TonAddress;
use crate::client::TonClientInterface;
use crate::tl::{InternalTransactionId, RawFullAccountState, SmcRunResult, TvmCell, TvmStackEntry};

mod error;
mod factory;
mod interface;
mod jetton;
mod latest_transactions_cache;
mod nft;
mod state;
mod wallet;

pub struct TonContract {
    factory: TonContractFactory,
    address: TonAddress,
}

impl TonContract {
    pub(crate) fn new(factory: &TonContractFactory, address: &TonAddress) -> TonContract {
        TonContract {
            factory: factory.clone(),
            address: address.clone(),
        }
    }

    pub async fn get_account_state(&self) -> Result<RawFullAccountState, TonContractError> {
        self.factory.get_account_state(&self.address).await
    }

    pub async fn get_state(&self) -> Result<TonContractState, TonContractError> {
        let r = self.factory.get_contract_state(&self.address).await?;
        Ok(r)
    }

    pub async fn get_state_by_transaction(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let r = self
            .factory
            .get_contract_state_by_transaction(&self.address, transaction_id)
            .await?;
        Ok(r)
    }
}

#[async_trait]
impl TonContractInterface for TonContract {
    fn client(&self) -> &dyn TonClientInterface {
        self.factory.get_client()
    }

    fn address(&self) -> &TonAddress {
        &self.address
    }

    async fn get_code_cell(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_code_cell().await?;
        Ok(result)
    }

    async fn get_data_cell(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_data_cell().await?;
        Ok(result)
    }

    async fn get_state_cell(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_state_cell().await?;
        Ok(result)
    }

    async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError> {
        let state = self.get_state().await?;
        let result = state.run_get_method(method, stack).await?;
        Ok(result)
    }
}
