use std::sync::Arc;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::config::MAINNET_CONFIG;
use crate::tl::TonNotification;

pub type TonNotificationReceiver = broadcast::Receiver<Arc<TonNotification>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TonConnectionParams {
    pub config: String,
    pub blockchain_name: Option<String>,
    pub use_callbacks_for_network: bool,
    pub ignore_cache: bool,
    pub keystore_dir: Option<String>,
}

impl Default for TonConnectionParams {
    fn default() -> Self {
        TonConnectionParams {
            config: MAINNET_CONFIG.to_string(),
            blockchain_name: None,
            use_callbacks_for_network: false,
            ignore_cache: false,
            keystore_dir: None,
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_CONNECTION_PARAMS: TonConnectionParams = TonConnectionParams::default();
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RetryStrategy {
    pub interval_ms: u64,
    pub max_retries: usize,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy {
            interval_ms: 5,
            max_retries: 10,
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_RETRY_STRATEGY: RetryStrategy = RetryStrategy::default();
}
