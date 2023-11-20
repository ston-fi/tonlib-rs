pub use builder::*;
#[cfg(feature = "state_cache")]
pub use cache::*;

mod builder;
#[cfg(feature = "state_cache")]
mod cache;

use crate::address::TonAddress;
use crate::client::{TonClient, TonClientInterface};
use crate::contract::{TonContract, TonContractError, TonContractState};
use crate::tl::{InternalTransactionId, RawFullAccountState};

#[cfg(feature = "state_cache")]
use std::time::Duration;

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
    ) -> Result<TonContractFactory, TonContractError> {
        let cache = if with_cache {
            let cache = ContractFactoryCache::new(client, capacity, time_to_live).await?;
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

    pub async fn get_contract_state(
        &self,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        #[cfg(feature = "state_cache")]
        if let Some(cache) = self.cache.as_ref() {
            cache.get_contract_state(&self.client, address).await
        } else {
            TonContractState::load(&self.client, address).await
        }
        #[cfg(not(feature = "state_cache"))]
        TonContractState::load(&self.client, address).await
    }

    pub async fn get_account_state(
        &self,
        account_address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        #[cfg(feature = "state_cache")]
        if let Some(cache) = self.cache.as_ref() {
            cache.get_account_state(&self.client, account_address).await
        } else {
            Ok(self.client.get_raw_account_state(account_address).await?)
        }
        #[cfg(not(feature = "state_cache"))]
        self.client.get_raw_account_state(account_address).await
    }

    pub async fn get_contract_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        TonContractState::load_by_transaction(&self.client, address, transaction_id).await
    }
}
