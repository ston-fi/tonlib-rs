use async_trait::async_trait;

pub use error::*;
pub use factory::*;
pub use interface::*;
pub use jetton::*;
pub use nft::*;
pub use state::*;
pub use wallet::*;

use crate::address::TonAddress;
use crate::client::TonClientInterface;
use crate::tl::{InternalTransactionId, SmcRunResult, TvmCell, TvmStackEntry};

mod error;
mod factory;
mod interface;
mod jetton;

mod nft;
mod state;
mod wallet;

pub struct TonContract {
    factory: TonContractFactory,
    address: TonAddress,
}

impl TonContract {
    pub(crate) fn new(factory: &TonContractFactory, address: &TonAddress) -> TonContract {
        let contract = TonContract {
            factory: factory.clone(),
            address: address.clone(),
        };
        contract
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

    async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    async fn get_data(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_data().await?;
        Ok(result)
    }

    async fn get_state(&self) -> Result<TvmCell, TonContractError> {
        let state = self.get_state().await?;
        let result = state.get_state().await?;
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
