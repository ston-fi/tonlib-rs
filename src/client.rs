mod builder;
mod connection;
mod error;
mod types;

pub use builder::*;
pub use connection::*;
pub use error::*;
use futures::{future::try_join_all, Future, FutureExt};
pub use types::*;

use async_trait::async_trait;
use rand::Rng;
use tokio::sync::Mutex;

use std::{fs, ops::Deref, path::Path, pin::Pin, sync::Arc};

use tokio_retry::{strategy::FixedInterval, RetryIf};

use crate::{address::TonAddress, tl::*};

pub struct TonClient {
    inner: Arc<Inner>,
}

struct Inner {
    retry_strategy: RetryStrategy,
    connections: Vec<PoolConnection>,
}

#[derive(Debug, Clone)]
pub struct TxData {
    pub account: TonAddress,
    pub internal_transaction_id: InternalTransactionId,
    pub raw_transaction: RawTransaction,
}

pub struct ShardTxData {
    pub shard: BlockIdExt,
    pub txs_data: Vec<TxData>,
}

impl TonClient {
    /// Creates a new TonClient
    pub async fn new(
        pool_size: usize,
        params: &TonConnectionParams,
        retry_strategy: &RetryStrategy,
        callback: Arc<dyn TonConnectionCallback + Send + Sync>,
    ) -> Result<TonClient, TonClientError> {
        let mut connections = Vec::with_capacity(pool_size);
        for i in 0..pool_size {
            let mut p = params.clone();
            if let Some(dir) = &params.keystore_dir {
                let keystore_prefix = Path::new(dir.as_str());
                let keystore_dir = keystore_prefix.join(format!("{}", i));
                fs::create_dir_all(&keystore_dir)?;
                let path_str = keystore_dir
                    .into_os_string()
                    .into_string()
                    .map_err(|_| TonClientError::InternalError)?;
                p.keystore_dir = Some(path_str)
            };
            let entry = PoolConnection {
                params: p,
                callback: callback.clone(),
                conn: Mutex::new(None),
            };
            connections.push(entry);
        }
        let inner = Inner {
            retry_strategy: retry_strategy.clone(),
            connections,
        };
        Ok(TonClient {
            inner: Arc::new(inner),
        })
    }

    pub fn builder() -> TonClientBuilder {
        TonClientBuilder::new()
    }

    pub async fn default() -> Result<TonClient, TonClientError> {
        Self::builder().build().await
    }

    async fn retrying_invoke(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        let fi = FixedInterval::from_millis(self.inner.retry_strategy.interval_ms);
        let strategy = fi.take(self.inner.retry_strategy.max_retries);
        let item = self.random_item();
        let result =
            RetryIf::spawn(strategy, || self.do_invoke(function, item), retry_condition).await;
        match result {
            Ok(result) => Ok(result),
            Err(e) => {
                item.reset().await;
                Err(e)
            }
        }
    }

    async fn do_invoke(
        &self,
        function: &TonFunction,
        item: &PoolConnection,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        let conn = item.get_connection().await?;
        let res = conn.invoke(function).await;
        match res {
            Ok(result) => Ok((conn, result)),
            Err(error) => Err(error),
        }
    }

    fn random_item(&self) -> &PoolConnection {
        let i = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..self.inner.connections.len())
        };
        let entry = &self.inner.connections[i];
        entry
    }

    pub fn set_log_verbosity_level(verbosity_level: u32) {
        TlTonClient::set_log_verbosity_level(verbosity_level)
    }

    pub async fn get_shard_transactions(
        &self,
        shard_id: &BlockIdExt,
    ) -> Result<Vec<TxData>, TonClientError> {
        let tx_ids = self.get_shard_tx_ids(shard_id).await?;
        let futures: Vec<Pin<Box<dyn Future<Output = Result<TxData, TonClientError>> + Send>>> =
            tx_ids
                .iter()
                .map(|tx_id| self.load_raw_tx(shard_id.workchain, tx_id).boxed())
                .collect();
        let txs: Vec<TxData> = try_join_all(futures).await?;
        Ok(txs)
    }

    pub async fn get_shards_transactions(
        &self,
        shards: &Vec<BlockIdExt>,
    ) -> Result<Vec<ShardTxData>, TonClientError> {
        let f = shards.iter().map(|shard| {
            self.get_shard_transactions(shard).map(move |txs_r| {
                txs_r.map(|txs| ShardTxData {
                    shard: shard.clone(),
                    txs_data: txs,
                })
            })
        });
        let txs: Vec<ShardTxData> = try_join_all(f).await?;
        Ok(txs)
    }

    async fn get_shard_tx_ids(
        &self,
        shard_ext: &BlockIdExt,
    ) -> Result<Vec<BlocksShortTxId>, TonClientError> {
        let mut after: BlocksAccountTransactionId = NULL_BLOCKS_ACCOUNT_TRANSACTION_ID.clone();
        let mut transactions: Vec<BlocksShortTxId> = Vec::new();
        loop {
            let mode = if after.lt == 0 { 7 } else { 128 + 7 };
            let txs: BlocksTransactions = self
                .get_block_transactions(&shard_ext, mode, 256, &after)
                .await?;
            if let Some(last) = txs.transactions.last() {
                after = BlocksAccountTransactionId {
                    account: last.account.clone(),
                    lt: last.lt,
                };
            }
            transactions.extend(txs.transactions);
            if !txs.incomplete {
                break;
            }
        }
        Ok(transactions)
    }

    async fn load_raw_tx(
        &self,
        workchain: i32,
        tx_id: &BlocksShortTxId,
    ) -> Result<TxData, TonClientError> {
        let addr = TonAddress::new(
            workchain,
            tx_id.account.as_slice().try_into().map_err(|_| {
                TonClientError::RawTransactionError {
                    msg: format!("Invalid TonAddress: {:?}", tx_id),
                }
            })?,
        );
        let id = InternalTransactionId {
            lt: tx_id.lt,
            hash: tx_id.hash.clone(),
        };
        let tx_result = self
            .get_raw_transactions_v2(addr.to_hex().as_str(), &id, 1, false)
            .await?;
        let tx = if tx_result.transactions.len() == 1 {
            tx_result.transactions[0].clone()
        } else {
            return Err(TonClientError::RawTransactionError {
                msg: format!(
                    "Expected 1 tx, got {}, query: {:?}/{:?}",
                    tx_result.transactions.len(),
                    addr,
                    id
                ),
            });
        };
        Ok(TxData {
            account: addr,
            internal_transaction_id: id,
            raw_transaction: tx,
        })
    }
}

#[async_trait]
impl TonFunctions for TonClient {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError> {
        let item = self.random_item();
        let conn = item.get_connection().await?;
        Ok(conn)
    }

    async fn invoke_on_connection(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        self.retrying_invoke(function).await
    }
}

impl Clone for TonClient {
    fn clone(&self) -> Self {
        TonClient {
            inner: self.inner.clone(),
        }
    }
}

fn maybe_error_code(error: &TonClientError) -> Option<i32> {
    if let TonClientError::TonlibError { code, .. } = error {
        Some(*code)
    } else {
        None
    }
}

fn retry_condition(error: &TonClientError) -> bool {
    if let Some(code) = maybe_error_code(error) {
        code == 500
    } else {
        false
    }
}

struct PoolConnection {
    params: TonConnectionParams,
    callback: Arc<dyn TonConnectionCallback + Send + Sync>,
    conn: Mutex<Option<TonConnection>>,
}

impl PoolConnection {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError> {
        let mut guard = self.conn.lock().await;
        match guard.deref() {
            Some(conn) => Ok(conn.clone()),
            None => {
                let conn = TonConnection::connect(&self.params, self.callback.clone()).await?;
                *guard = Some(conn.clone());
                Ok(conn)
            }
        }
    }

    #[allow(dead_code)]
    async fn reset(&self) {
        let mut guard = self.conn.lock().await;
        *guard = None;
    }
}
