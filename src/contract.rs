mod error;
mod interface;
mod jetton;
mod latest_transactions_cache;
mod nft;
mod state;
mod wallet;

use async_trait::async_trait;
pub use error::*;
pub use interface::*;
pub use jetton::*;
pub use latest_transactions_cache::*;
pub use nft::*;
pub use state::*;
pub use wallet::*;

use crate::address::TonAddress;
use crate::client::{TonClient, TonClientInterface};
use crate::tl::{
    FullAccountState, InternalTransactionId, RawFullAccountState, RawTransaction, RawTransactions,
    SmcRunResult, TvmCell, TvmStackEntry,
};

pub struct TonContract {
    client: TonClient,
    address: TonAddress,
}

impl TonContract {
    pub fn new(client: &TonClient, address: &TonAddress) -> TonContract {
        let contract = TonContract {
            client: client.clone(),
            address: address.clone(),
        };
        contract
    }

    pub async fn load_state(&self) -> Result<TonContractState, TonContractError> {
        let state = TonContractState::load(&self.client.clone(), &self.address).await?;
        Ok(state)
    }

    pub async fn load_state_by_transaction_id(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let state = TonContractState::load_by_transaction_id(
            &self.client.clone(),
            &self.address,
            transaction_id,
        )
        .await?;
        Ok(state)
    }

    pub async fn get_account_state(&self) -> Result<FullAccountState, TonContractError> {
        self.client
            .get_account_state(self.address())
            .await
            .map_err(|error| {
                TonContractError::client_method_error(
                    "get_account_state",
                    Some(&self.address),
                    error,
                )
            })
    }

    pub async fn get_raw_account_state(&self) -> Result<RawFullAccountState, TonContractError> {
        self.client
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
        self.client
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
    ) -> LatestContractTransactionsCache {
        LatestContractTransactionsCache::new(&self.client, &self.address, capacity)
    }
}

#[async_trait]
impl TonContractInterface for TonContract {
    fn client(&self) -> &dyn TonClientInterface {
        &self.client
    }

    fn address(&self) -> &TonAddress {
        &self.address
    }

    async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        let state = self.load_state().await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    async fn get_data(&self) -> Result<TvmCell, TonContractError> {
        let state = self.load_state().await?;
        let result = state.get_data().await?;
        Ok(result)
    }

    async fn get_state(&self) -> Result<TvmCell, TonContractError> {
        let state = self.load_state().await?;
        let result = state.get_state().await?;
        Ok(result)
    }

    async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError> {
        let state = self.load_state().await?;
        let result = state.run_get_method(method, stack).await?;
        Ok(result)
    }
}
