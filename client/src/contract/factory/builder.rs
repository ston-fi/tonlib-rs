use std::sync::Arc;
use std::time::Duration;

use super::{DefaultLibraryLoader, LibraryLoader, LibraryProvider};
use crate::client::TonClient;
use crate::contract::{TonContractError, TonContractFactory};

pub struct TonContractFactoryBuilder {
    client: TonClient,
    with_cache: bool,
    account_state_cache_capacity: u64,
    account_state_cache_time_to_live: Duration,
    txid_cache_capacity: u64,
    txid_cache_time_to_live: Duration,
    presync_blocks: i32,
    library_provider: LibraryProvider,
}

impl TonContractFactoryBuilder {
    const DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_ACCOUNT_STATE_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

    const DEFAULT_TXID_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_TXID_STATE_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

    const DEFAULT_PRESYNC_BLOCKS: i32 = 50;

    pub(crate) fn new(client: &TonClient) -> Self {
        let loader = DefaultLibraryLoader::new(client);
        let library_provider = LibraryProvider::new(loader);
        TonContractFactoryBuilder {
            client: client.clone(),
            with_cache: false,
            account_state_cache_capacity: 0,
            account_state_cache_time_to_live: Duration::default(),
            txid_cache_capacity: 0,
            txid_cache_time_to_live: Duration::default(),
            presync_blocks: Self::DEFAULT_PRESYNC_BLOCKS,
            library_provider,
        }
    }

    pub fn with_cache(
        mut self,
        txid_cache_capacity: u64,
        txid_cache_time_to_live: Duration,
        account_state_cache_capacity: u64,
        account_state_cache_time_to_live: Duration,
    ) -> Self {
        self.with_cache = true;
        self.txid_cache_capacity = txid_cache_capacity;
        self.txid_cache_time_to_live = txid_cache_time_to_live;
        self.account_state_cache_capacity = account_state_cache_capacity;
        self.account_state_cache_time_to_live = account_state_cache_time_to_live;
        self
    }

    pub fn with_default_cache(mut self) -> Self {
        self.with_cache = true;
        self.account_state_cache_capacity = Self::DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY;
        self.account_state_cache_time_to_live = Self::DEFAULT_ACCOUNT_STATE_CACHE_TTL;
        self.txid_cache_capacity = Self::DEFAULT_TXID_CACHE_CAPACITY;
        self.txid_cache_time_to_live = Self::DEFAULT_TXID_STATE_CACHE_TTL;
        self
    }

    pub fn with_presync_blocks(mut self, presync_blocks: i32) -> Self {
        self.presync_blocks = presync_blocks;
        self
    }

    pub async fn build(&self) -> Result<TonContractFactory, TonContractError> {
        TonContractFactory::new(
            &self.client,
            self.with_cache,
            self.account_state_cache_capacity,
            self.account_state_cache_time_to_live,
            self.txid_cache_capacity,
            self.txid_cache_time_to_live,
            self.presync_blocks,
            self.library_provider.clone(),
        )
        .await
    }
    pub fn with_library_loader(mut self, library_loader: &Arc<dyn LibraryLoader>) -> Self {
        let library_provider = LibraryProvider::new(library_loader.clone());
        self.library_provider = library_provider;
        self
    }

    pub fn with_library_provider(mut self, library_provider: &LibraryProvider) -> Self {
        self.library_provider = library_provider.clone();
        self
    }
}
