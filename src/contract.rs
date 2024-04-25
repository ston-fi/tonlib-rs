use std::sync::Arc;

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
use crate::tl::{InternalTransactionId, RawFullAccountState};
use crate::types::{TonMethodId, TvmStackEntry, TvmSuccess};

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

    pub async fn get_account_state_by_transaction(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<RawFullAccountState, TonContractError> {
        self.factory
            .get_account_state_by_transaction(&self.address, transaction_id)
            .await
    }

    pub async fn get_state(&self) -> Result<TonContractState, TonContractError> {
        let r = self
            .factory
            .get_latest_contract_state(&self.address)
            .await?;
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
    fn factory(&self) -> &TonContractFactory {
        &self.factory
    }

    fn address(&self) -> &TonAddress {
        &self.address
    }

    async fn get_account_state(&self) -> Result<Arc<RawFullAccountState>, TonContractError> {
        self.factory.get_latest_account_state(self.address()).await
    }

    async fn run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        let state = self.get_state().await?;
        let result = state.run_get_method(method, stack).await?;
        Ok(result)
    }
}
