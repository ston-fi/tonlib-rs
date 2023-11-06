use crate::address::TonAddress;
use crate::client::TonClient;
use crate::contract::{TonContract, TonContractError, TonContractState};
use crate::tl::InternalTransactionId;
use std::sync::Arc;

#[derive(Clone)]
pub struct TonContractFactory {
    inner: Arc<Inner>,
}

pub struct Inner {
    client: TonClient,
    // TODO: Add cache of contract states
}

impl TonContractFactory {
    pub fn new(client: &TonClient) -> TonContractFactory {
        let inner = Inner {
            client: client.clone(),
        };
        TonContractFactory {
            inner: Arc::new(inner),
        }
    }

    pub fn get_client(&self) -> &TonClient {
        &self.inner.client
    }

    pub fn get_contract(&self, address: &TonAddress) -> TonContract {
        TonContract::new(self, address)
    }

    pub async fn get_contract_state(
        &self,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        // TODO: Load in the cache
        TonContractState::load(&self.inner.client, address).await
    }

    pub async fn get_contract_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        TonContractState::load_by_transaction(&self.inner.client, address, transaction_id).await
    }
}
