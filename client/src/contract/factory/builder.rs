use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use std::time::Duration;

use super::{BlockchainLibraryProvider, LibraryProvider};
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
    library_provider: Arc<dyn LibraryProvider>,
    current_seqno: Arc<AtomicI32>,
    max_dyn_libs_per_contract: usize,
}

impl TonContractFactoryBuilder {
    const DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_ACCOUNT_STATE_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

    const DEFAULT_TXID_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_TXID_STATE_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

    const DEFAULT_PRESYNC_BLOCKS: i32 = 50;

    pub(crate) fn new(client: &TonClient) -> Self {
        let current_seqno_counter: Arc<AtomicI32> = Arc::new(0.into());

        let library_provider = Arc::new(BlockchainLibraryProvider::new(client, None));
        TonContractFactoryBuilder {
            client: client.clone(),
            with_cache: false,
            account_state_cache_capacity: 0,
            account_state_cache_time_to_live: Duration::default(),
            txid_cache_capacity: 0,
            txid_cache_time_to_live: Duration::default(),
            presync_blocks: Self::DEFAULT_PRESYNC_BLOCKS,
            library_provider,
            current_seqno: current_seqno_counter,
            max_dyn_libs_per_contract: 100,
        }
    }

    pub fn with_cache(
        &mut self,
        txid_cache_capacity: u64,
        txid_cache_time_to_live: Duration,
        account_state_cache_capacity: u64,
        account_state_cache_time_to_live: Duration,
    ) -> &mut Self {
        self.with_cache = true;
        self.txid_cache_capacity = txid_cache_capacity;
        self.txid_cache_time_to_live = txid_cache_time_to_live;
        self.account_state_cache_capacity = account_state_cache_capacity;
        self.account_state_cache_time_to_live = account_state_cache_time_to_live;
        self
    }

    pub fn with_default_cache(&mut self) -> &mut Self {
        self.with_cache = true;
        self.account_state_cache_capacity = Self::DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY;
        self.account_state_cache_time_to_live = Self::DEFAULT_ACCOUNT_STATE_CACHE_TTL;
        self.txid_cache_capacity = Self::DEFAULT_TXID_CACHE_CAPACITY;
        self.txid_cache_time_to_live = Self::DEFAULT_TXID_STATE_CACHE_TTL;
        self
    }

    pub fn with_presync_blocks(&mut self, presync_blocks: i32) -> &mut Self {
        self.presync_blocks = presync_blocks;
        self
    }

    pub fn with_max_dyn_libs_per_contract(&mut self, max: usize) -> &mut Self {
        self.max_dyn_libs_per_contract = max;
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
            self.current_seqno.clone(),
            self.max_dyn_libs_per_contract,
        )
        .await
    }

    pub fn with_library_provider(
        &mut self,
        library_provider: Arc<dyn LibraryProvider>,
    ) -> &mut Self {
        self.library_provider = library_provider.clone();
        self
    }
}
