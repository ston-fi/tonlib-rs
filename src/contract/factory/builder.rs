#[cfg(feature = "state_cache")]
use std::time::Duration;

use crate::client::TonClient;
use crate::contract::{TonContractError, TonContractFactory};

#[cfg(feature = "state_cache")]
pub struct TonContractFactoryBuilder {
    client: TonClient,
    with_cache: bool,
    account_state_cache_capacity: u64,
    account_state_cache_time_to_live: Duration,
    smc_state_cache_capacity: u64,
    smc_state_cache_time_to_live: Duration,
    txid_cache_capacity: u64,
    txid_cache_time_to_live: Duration,
    presync_blocks: i32,
}

#[cfg(feature = "state_cache")]
impl TonContractFactoryBuilder {
    const DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_ACCOUNT_STATE_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

    const DEFAULT_SMC_STATE_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_SMC_STATE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

    const DEFAULT_TXID_CACHE_CAPACITY: u64 = 100_000;
    const DEFAULT_TXID_STATE_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

    const DEFAULT_PRESYNC_BLOCKS: i32 = 50;

    pub(crate) fn new(client: &TonClient) -> Self {
        TonContractFactoryBuilder {
            client: client.clone(),
            with_cache: false,
            account_state_cache_capacity: 0,
            account_state_cache_time_to_live: Duration::default(),
            smc_state_cache_capacity: 0,
            smc_state_cache_time_to_live: Duration::default(),
            txid_cache_capacity: 0,
            txid_cache_time_to_live: Duration::default(),
            presync_blocks: Self::DEFAULT_PRESYNC_BLOCKS,
        }
    }

    pub fn with_account_state_cache(
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

    pub fn with_state_cache(
        &mut self,
        smc_state_cache_capacity: u64,
        smc_state_cache_time_to_live: Duration,
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
        self.smc_state_cache_capacity = smc_state_cache_capacity;
        self.smc_state_cache_time_to_live = smc_state_cache_time_to_live;
        self
    }

    pub fn with_default_cache(&mut self) -> &mut Self {
        self.with_cache = true;
        self.account_state_cache_capacity = Self::DEFAULT_ACCOUNT_STATE_CACHE_CAPACITY;
        self.account_state_cache_time_to_live = Self::DEFAULT_ACCOUNT_STATE_CACHE_TTL;
        self.smc_state_cache_capacity = Self::DEFAULT_SMC_STATE_CACHE_CAPACITY;
        self.smc_state_cache_time_to_live = Self::DEFAULT_SMC_STATE_CACHE_TTL;
        self.txid_cache_capacity = Self::DEFAULT_TXID_CACHE_CAPACITY;
        self.txid_cache_time_to_live = Self::DEFAULT_TXID_STATE_CACHE_TTL;
        self
    }

    pub fn presync_blocks(&mut self, presync_blocks: i32) -> &mut Self {
        self.presync_blocks = presync_blocks;
        self
    }

    pub async fn build(&self) -> Result<TonContractFactory, TonContractError> {
        TonContractFactory::new(
            &self.client,
            self.with_cache,
            self.account_state_cache_capacity,
            self.account_state_cache_time_to_live,
            self.smc_state_cache_capacity,
            self.smc_state_cache_time_to_live,
            self.txid_cache_capacity,
            self.txid_cache_time_to_live,
            self.presync_blocks,
        )
        .await
    }
}

#[cfg(not(feature = "state_cache"))]
pub struct TonContractFactoryBuilder {
    client: TonClient,
}

#[cfg(not(feature = "state_cache"))]
impl TonContractFactoryBuilder {
    pub(crate) fn new(client: &TonClient) -> Self {
        TonContractFactoryBuilder {
            client: client.clone(),
        }
    }
    pub async fn build(&self) -> Result<TonContractFactory, TonContractError> {
        TonContractFactory::new(&self.client).await
    }
}
