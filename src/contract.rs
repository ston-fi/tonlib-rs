mod error;
mod jetton;
mod nft;
mod ton;
mod wallet;

pub use error::*;
pub use jetton::*;
pub use nft::*;
pub use ton::*;
pub use wallet::*;

use crate::{
    address::TonAddress,
    client::{TonClient, TonFunctions},
    tl::{
        FullAccountState, InternalTransactionId, RawFullAccountState, RawTransaction,
        RawTransactions, SmcRunResult, TvmCell, TvmStackEntry,
    },
};

pub struct TonContract {
    client: TonClient,
    address: TonAddress,
    address_hex: String,
}

impl TonContract {
    pub fn new(client: &TonClient, address: &TonAddress) -> TonContract {
        let contract = TonContract {
            client: client.clone(),
            address: address.clone(),
            address_hex: address.to_hex(),
        };
        contract
    }

    #[inline(always)]
    pub fn client(&self) -> &TonClient {
        &self.client
    }

    #[inline(always)]
    pub fn address(&self) -> &TonAddress {
        &self.address
    }

    #[inline(always)]
    pub fn address_hex(&self) -> &str {
        self.address_hex.as_str()
    }

    pub async fn load_state(&self) -> Result<TonContractState, TonContractError> {
        let state = TonContractState::load(&self.client, &self.address).await?;
        Ok(state)
    }

    pub async fn load_state_by_transaction_id(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let state =
            TonContractState::load_by_transaction_id(&self.client, &self.address, transaction_id)
                .await?;
        Ok(state)
    }

    pub async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        let state = self.load_state().await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    pub async fn get_code_by_transaction_id(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> Result<TvmCell, TonContractError> {
        let state = self.load_state_by_transaction_id(transaction_id).await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    pub async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError> {
        let state = self.load_state().await?;
        let result = state.run_get_method(method, stack).await?;
        Ok(result)
    }

    pub async fn get_account_state(&self) -> Result<FullAccountState, TonContractError> {
        self.client
            .get_account_state(self.address_hex())
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
            .get_raw_account_state(self.address_hex())
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
            .get_raw_transactions_v2(self.address_hex(), from_transaction_id, limit, false)
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

    pub async fn create_contract_transactions_cache(
        &self,
        capacity: usize,
    ) -> LatestContractTransactionsCache {
        LatestContractTransactionsCache::new(&self.client, &self.address, capacity)
    }
}
