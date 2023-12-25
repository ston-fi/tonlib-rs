use std::time::Duration;

use crate::client::TonClient;
use crate::contract::{TonContractError, TonContractFactory};

#[cfg(feature = "state_cache")]
const DEFAULT_CAPACITY: u64 = 100_000;
#[cfg(feature = "state_cache")]
const DEFAULT_TTL: Duration = Duration::from_secs(60 * 60);
#[cfg(feature = "state_cache")]
const DEFAULT_PRESYNC_BLOCKS: i32 = 50;

pub struct TonContractFactoryBuilder {
    client: TonClient,
    #[cfg(feature = "state_cache")]
    with_cache: bool,
    #[cfg(feature = "state_cache")]
    capacity: u64,
    #[cfg(feature = "state_cache")]
    time_to_live: Duration,
    #[cfg(feature = "state_cache")]
    presync_blocks: i32,
}

impl TonContractFactoryBuilder {
    #[cfg(feature = "state_cache")]
    pub(crate) fn new(client: &TonClient) -> Self {
        TonContractFactoryBuilder {
            client: client.clone(),
            with_cache: false,
            capacity: 0,
            time_to_live: Duration::default(),
            presync_blocks: DEFAULT_PRESYNC_BLOCKS,
        }
    }

    #[cfg(not(feature = "state_cache"))]
    pub(crate) fn new(client: &TonClient) -> Self {
        TonContractFactoryBuilder {
            client: client.clone(),
        }
    }

    #[cfg(feature = "state_cache")]
    pub fn with_cache(&mut self, capacity: u64, time_to_live: Duration) -> &mut Self {
        self.with_cache = true;
        self.capacity = capacity;
        self.time_to_live = time_to_live;
        self
    }

    #[cfg(not(feature = "state_cache"))]
    pub fn with_cache(&mut self, _capacity: u64, _time_to_live: Duration) -> &mut Self {
        panic!("State cache disabled. Use feature flag \"state_cache\" to enable it.  ");
    }

    #[cfg(feature = "state_cache")]
    pub fn with_default_cache(&mut self) -> &mut Self {
        self.with_cache = true;
        self.capacity = DEFAULT_CAPACITY;
        self.time_to_live = DEFAULT_TTL;
        self
    }

    #[cfg(not(feature = "state_cache"))]
    pub fn with_default_cache(&mut self) -> &mut Self {
        panic!("State cache disabled. Use feature flag \"state_cache\" to enable it.  ");
    }

    #[cfg(feature = "state_cache")]
    pub async fn build(&self) -> Result<TonContractFactory, TonContractError> {
        TonContractFactory::new(
            &self.client,
            self.with_cache.clone(),
            self.capacity,
            self.time_to_live,
            self.presync_blocks,
        )
        .await
    }
    #[cfg(feature = "state_cache")]
    pub fn presync_blocks(&mut self, presync_blocks: i32) -> &mut Self {
        self.presync_blocks = presync_blocks;
        self
    }

    #[cfg(not(feature = "state_cache"))]
    pub async fn build(&self) -> Result<TonContractFactory, TonContractError> {
        TonContractFactory::new(&self.client).await
    }
}
