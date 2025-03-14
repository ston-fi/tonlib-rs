use std::sync::Arc;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tonlib_core::{TonAddress, TonHash};

use super::{
    BlocksShortTxId, TonClientError, DEFAULT_CONNECTION_CONCURRENCY_LIMIT,
    DEFAULT_NOTIFICATION_QUEUE_LENGTH, DEFAULT_UPDATE_INIT_BLOCK,
};
use crate::config::MAINNET_CONFIG;
use crate::tl::{InternalTransactionId, TonNotification};

pub type TonNotificationReceiver = broadcast::Receiver<Arc<TonNotification>>;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct TxId {
    pub address: TonAddress,
    pub internal_transaction_id: InternalTransactionId,
}

impl TxId {
    pub fn new(workchain: i32, tx_id: &BlocksShortTxId) -> Result<TxId, TonClientError> {
        let hash_part = TonHash::try_from(tx_id.account.as_slice())?;
        let addr = TonAddress::new(workchain, hash_part);
        let id = InternalTransactionId {
            lt: tx_id.lt,
            hash: tx_id.hash.clone(),
        };
        Ok(TxId {
            address: addr,
            internal_transaction_id: id,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TonConnectionParams {
    pub config: String,
    #[serde(default)]
    pub blockchain_name: Option<String>,
    #[serde(default)]
    pub use_callbacks_for_network: bool,
    #[serde(default)]
    pub ignore_cache: bool,
    #[serde(default)]
    pub keystore_dir: Option<String>,
    #[serde(default = "default_notification_queue_length")]
    pub notification_queue_length: usize,
    #[serde(default = "default_connection_concurrency_limit")]
    pub concurrency_limit: usize,
    #[serde(default = "default_update_init_block")]
    pub update_init_block: bool,
}

impl Default for TonConnectionParams {
    fn default() -> Self {
        TonConnectionParams {
            config: MAINNET_CONFIG.to_string(),
            blockchain_name: None,
            use_callbacks_for_network: false,
            ignore_cache: false,
            keystore_dir: None,
            notification_queue_length: DEFAULT_NOTIFICATION_QUEUE_LENGTH,
            concurrency_limit: DEFAULT_CONNECTION_CONCURRENCY_LIMIT,
            update_init_block: DEFAULT_UPDATE_INIT_BLOCK,
        }
    }
}
fn default_notification_queue_length() -> usize {
    DEFAULT_NOTIFICATION_QUEUE_LENGTH
}

fn default_connection_concurrency_limit() -> usize {
    DEFAULT_CONNECTION_CONCURRENCY_LIMIT
}

fn default_update_init_block() -> bool {
    DEFAULT_UPDATE_INIT_BLOCK
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
