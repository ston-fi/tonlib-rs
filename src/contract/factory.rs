pub use builder::*;
#[cfg(feature = "state_cache")]
pub use cache::*;

mod builder;
#[cfg(feature = "state_cache")]
mod cache;

#[cfg(feature = "state_cache")]
use std::time::Duration;

use crate::address::TonAddress;
use crate::client::{TonClient, TonClientInterface};
use crate::contract::{TonContract, TonContractError, TonContractState};
use crate::tl::{InternalTransactionId, RawFullAccountState};

#[derive(Clone)]
pub struct TonContractFactory {
    client: TonClient,
    #[cfg(feature = "state_cache")]
    cache: Option<ContractFactoryCache>,
}

impl TonContractFactory {
    pub fn builder(client: &TonClient) -> TonContractFactoryBuilder {
        TonContractFactoryBuilder::new(client)
    }

    #[cfg(feature = "state_cache")]
    pub(crate) async fn new(
        client: &TonClient,
        with_cache: bool,
        capacity: u64,
        time_to_live: Duration,
        presync_blocks: i32,
    ) -> Result<TonContractFactory, TonContractError> {
        let cache = if with_cache {
            let cache =
                ContractFactoryCache::new(client, capacity, time_to_live, presync_blocks).await?;
            Some(cache)
        } else {
            None
        };

        Ok(TonContractFactory {
            client: client.clone(),
            cache,
        })
    }
    #[cfg(not(feature = "state_cache"))]
    pub(crate) async fn new(client: &TonClient) -> Result<TonContractFactory, TonContractError> {
        Ok(TonContractFactory {
            client: client.clone(),
        })
    }

    pub fn get_client(&self) -> &TonClient {
        &self.client
    }

    pub fn get_contract(&self, address: &TonAddress) -> TonContract {
        TonContract::new(self, address)
    }

    pub async fn get_account_state(
        &self,
        address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        #[cfg(feature = "state_cache")]
        if let Some(cache) = self.cache.as_ref() {
            cache.get_account_state(address).await
        } else {
            Ok(self.client.get_raw_account_state(address).await?)
        }
        #[cfg(not(feature = "state_cache"))]
        Ok(self.client.get_raw_account_state(address).await?)
    }

    pub async fn get_account_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<RawFullAccountState, TonContractError> {
        let state = self
            .client
            .get_raw_account_state_by_transaction(address, transaction_id)
            .await?;
        Ok(state)
    }

    pub async fn get_contract_state(
        &self,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        #[cfg(feature = "state_cache")]
        if let Some(cache) = self.cache.as_ref() {
            cache.get_contract_state(address).await
        } else {
            TonContractState::load(&self.client, address).await
        }
        #[cfg(not(feature = "state_cache"))]
        TonContractState::load(&self.client, address).await
    }

    pub async fn get_contract_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        TonContractState::load_by_transaction(&self.client, address, transaction_id).await
    }
}
