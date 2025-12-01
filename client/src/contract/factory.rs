use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use std::time::Duration;

pub use blockchain_library_provider::*;
pub use builder::*;
pub use cache::*;
pub use library_provider::*;
use tokio::sync::OnceCell;
use tonlib_core::TonAddress;

use crate::client::{TonClient, TonClientError, TonClientInterface};
use crate::contract::{LoadedSmcState, TonContract, TonContractError, TonContractState};
use crate::tl::{ConfigInfo, InternalTransactionId, RawFullAccountState};

mod blockchain_library_provider;
mod builder;
mod cache;
mod library_provider;

#[derive(Clone)]
pub struct TonContractFactory {
    inner: Arc<Inner>,
}

struct Inner {
    client: TonClient,
    config_info: OnceCell<ConfigInfo>,
    library_provider: Arc<dyn LibraryProvider>,
    cache: Option<ContractFactoryCache>,
    max_libs_per_contract: usize,
}

impl TonContractFactory {
    pub fn builder(client: &TonClient) -> TonContractFactoryBuilder {
        TonContractFactoryBuilder::new(client)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        client: &TonClient,
        with_cache: bool,
        account_state_cache_capacity: u64,
        account_state_cache_time_to_live: Duration,
        txid_cache_capacity: u64,
        txid_cache_time_to_live: Duration,
        presync_blocks: i32,
        library_provider: Arc<dyn LibraryProvider>,
        current_seqno: Arc<AtomicI32>,
        max_libs_per_contract: usize,
    ) -> Result<TonContractFactory, TonContractError> {
        let cache = if with_cache {
            let cache = ContractFactoryCache::new(
                client,
                account_state_cache_capacity,
                account_state_cache_time_to_live,
                txid_cache_capacity,
                txid_cache_time_to_live,
                presync_blocks,
                current_seqno,
            )
            .await?;
            Some(cache)
        } else {
            None
        };
        let config_info = OnceCell::const_new();
        let inner = Inner {
            client: client.clone(),
            config_info,
            cache,
            library_provider,
            max_libs_per_contract,
        };

        Ok(TonContractFactory {
            inner: Arc::new(inner),
        })
    }

    pub fn client(&self) -> &TonClient {
        &self.inner.client
    }

    pub async fn get_config_cell_serial(&self) -> Result<&[u8], TonClientError> {
        let c = self
            .inner
            .config_info
            .get_or_try_init(|| self.client().get_config_all(0))
            .await?;
        Ok(c.config.bytes.as_slice())
    }

    pub fn library_provider(&self) -> Arc<dyn LibraryProvider> {
        self.inner.library_provider.clone()
    }

    pub fn max_libs_per_contract(&self) -> usize {
        self.inner.max_libs_per_contract
    }

    pub fn get_contract(&self, address: &TonAddress) -> TonContract {
        TonContract::new(self, address)
    }

    pub async fn get_latest_account_state(
        &self,
        address: &TonAddress,
    ) -> Result<Arc<RawFullAccountState>, TonContractError> {
        if let Some(cache) = self.inner.cache.as_ref() {
            cache.get_account_state(address).await
        } else {
            Ok(Arc::new(
                self.client().get_raw_account_state(address).await?,
            ))
        }
    }

    pub async fn get_account_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<RawFullAccountState, TonContractError> {
        let state = self
            .inner
            .client
            .get_raw_account_state_by_transaction(address, transaction_id)
            .await?;
        Ok(state)
    }

    pub async fn get_smc_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<Arc<LoadedSmcState>, TonContractError> {
        if let Some(cache) = self.inner.cache.as_ref() {
            cache
                .get_smc_state_by_transaction(address, transaction_id)
                .await
        } else {
            Ok(Arc::new(
                self.client()
                    .smc_load_by_transaction(address, transaction_id)
                    .await?,
            ))
        }
    }

    pub async fn get_latest_contract_state(
        &self,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let account_state = Arc::new(self.get_latest_account_state(address).await?);
        let contract_state = TonContractState::new(self, address, &account_state);
        Ok(contract_state)
    }

    pub async fn get_contract_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let account_state = Arc::new(
            self.get_account_state_by_transaction(address, transaction_id)
                .await?,
        );
        let contract_state = TonContractState::new(self, address, &account_state);
        Ok(contract_state)
    }

    pub fn get_factory_cache_stats(&self) -> ContractFactoryCacheStats {
        if let Some(cache) = &self.inner.cache {
            cache.get_cache_stats()
        } else {
            ContractFactoryCacheStats::default()
        }
    }
}
