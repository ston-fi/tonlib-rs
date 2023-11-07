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
use crate::tl::{
    FullAccountState, InternalTransactionId, RawFullAccountState, RawTransaction, RawTransactions,
    SmcRunResult, TvmCell, TvmStackEntry,
};

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

    pub async fn get_account_state(&self) -> Result<FullAccountState, TonContractError> {
        self.factory
            .get_client()
            .get_account_state(self.address())
            .await
            .map_client_error("get_account_state", &self.address)
    }

    pub async fn get_raw_account_state(&self) -> Result<RawFullAccountState, TonContractError> {
        self.factory
            .get_client()
            .get_raw_account_state(self.address())
            .await
            .map_err(|error| {
                TonContractError::client_method_error(
                    "get_raw_account_state",
                    Some(&self.address),
                    error,
                )
            })
    }

    pub async fn get_raw_transactions(
        &self,
        from_transaction_id: &InternalTransactionId,
        limit: usize,
    ) -> Result<RawTransactions, TonContractError> {
        self.factory
            .get_client()
            .get_raw_transactions_v2(self.address(), from_transaction_id, limit, false)
            .await
            .map_err(|error| {
                TonContractError::client_method_error(
                    "get_raw_transactions_v2",
                    Some(&self.address),
                    error,
                )
            })
    }

    pub async fn get_raw_transaction(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<Option<RawTransaction>, TonContractError> {
        let txs = self.get_raw_transactions(transaction_id, 1).await?;
        match txs.transactions.len() {
            0 => Ok(None),
            1 => Ok(Some(txs.transactions[0].clone())),
            n => Err(TonContractError::InternalError {
                message: format!("expected one transaction for {} got {}", transaction_id, n),
            }),
        }
    }

    pub async fn create_latest_transactions_cache(
        &self,
        capacity: usize,
        soft_limit: bool,
    ) -> LatestContractTransactionsCache {
        LatestContractTransactionsCache::new(
            &self.factory.get_client(),
            &self.address,
            capacity,
            soft_limit,
        )
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
