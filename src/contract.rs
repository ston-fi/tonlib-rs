mod state;

use crate::{address::TonAddress, tl::stack::TvmCell};
use anyhow::anyhow;
use std::error::Error;
use std::fmt;

use crate::client::{TonClient, TonFunctions};
use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{
    FullAccountState, InternalTransactionId, RawFullAccountState, RawTransaction, RawTransactions,
    SmcRunResult,
};

pub use state::TonContractState;

#[derive(Debug, Clone)]
pub struct TonContractError {
    pub gas_used: i64,
    pub stack: Vec<TvmStackEntry>,
    pub exit_code: i32,
}

impl fmt::Display for TonContractError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TonContractError code: {}, gas: {}, stack: {:?}",
            self.exit_code, self.gas_used, self.stack
        )
    }
}

impl Error for TonContractError {}

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

    pub async fn load_state(&self) -> anyhow::Result<TonContractState> {
        let state = TonContractState::load(&self.client, &self.address).await?;
        Ok(state)
    }

    pub async fn load_state_by_transaction_id(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> anyhow::Result<TonContractState> {
        let state =
            TonContractState::load_by_transaction_id(&self.client, &self.address, transaction_id)
                .await?;
        Ok(state)
    }

    pub async fn get_code(&self) -> anyhow::Result<TvmCell> {
        let state = self.load_state().await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    pub async fn get_code_by_transaction_id(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> anyhow::Result<TvmCell> {
        let state = self.load_state_by_transaction_id(transaction_id).await?;
        let result = state.get_code().await?;
        Ok(result)
    }

    pub async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> anyhow::Result<SmcRunResult> {
        let state = self.load_state().await?;
        let result = state.run_get_method(method, stack).await?;
        Ok(result)
    }

    pub async fn get_account_state(&self) -> anyhow::Result<FullAccountState> {
        self.client.get_account_state(self.address_hex()).await
    }

    pub async fn get_raw_account_state(&self) -> anyhow::Result<RawFullAccountState> {
        self.client.get_raw_account_state(self.address_hex()).await
    }

    pub async fn get_raw_transactions(
        &self,
        from_transaction_id: &InternalTransactionId,
        limit: usize,
    ) -> anyhow::Result<RawTransactions> {
        self.client
            .get_raw_transactions_v2(self.address_hex(), from_transaction_id, limit, false)
            .await
    }

    pub async fn get_raw_transaction(
        &self,
        transaction_id: &InternalTransactionId,
    ) -> anyhow::Result<Option<RawTransaction>> {
        let txs = self.get_raw_transactions(transaction_id, 1).await?;
        match txs.transactions.len() {
            0 => Ok(None),
            1 => Ok(Some(txs.transactions[0].clone())),
            _ => Err(anyhow!(
                "Error getting tx {}: expected single tx, got {}",
                transaction_id,
                txs.transactions.len()
            )),
        }
    }
}
