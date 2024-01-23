use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;
use tokio::sync::Mutex;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::RetryIf;

pub use block_functions::*;
pub use block_stream::*;
pub use builder::*;
pub use callback::*;
pub use connection::*;
pub use error::*;
pub use interface::*;

pub use types::*;

use crate::tl::*;

mod block_functions;
mod block_stream;
mod builder;
mod callback;
mod connection;
mod error;
mod interface;

mod types;

pub struct TonClient {
    inner: Arc<Inner>,
}

struct Inner {
    retry_strategy: RetryStrategy,
    connections: Vec<PoolConnection>,
}

impl TonClient {
    /// Creates a new TonClient
    pub async fn new(
        pool_size: usize,
        params: &TonConnectionParams,
        retry_strategy: &RetryStrategy,
        callback: Arc<dyn TonConnectionCallback>,
        archive_nodes_only: bool,
    ) -> Result<TonClient, TonClientError> {
        let mut connections = Vec::with_capacity(pool_size);
        for i in 0..pool_size {
            let mut p = params.clone();
            if let Some(dir) = &params.keystore_dir {
                let keystore_prefix = Path::new(dir.as_str());
                let keystore_dir = keystore_prefix.join(format!("{}", i));
                fs::create_dir_all(&keystore_dir)?;
                let path_str = keystore_dir.into_os_string().into_string().map_err(|_| {
                    TonClientError::InternalError {
                        message: "Error constructing keystore path".to_string(),
                    }
                })?;
                p.keystore_dir = Some(path_str)
            };
            let entry = PoolConnection {
                params: p,
                callback: callback.clone(),
                conn: Mutex::new(None),
                archive_nodes_only,
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
        RetryIf::spawn(strategy, || self.do_invoke(function), retry_condition).await
    }

    async fn do_invoke(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        let item = self.random_item();
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
        &self.inner.connections[i] as _
    }

    pub fn set_log_verbosity_level(verbosity_level: u32) {
        TlTonClient::set_log_verbosity_level(verbosity_level)
    }
}

#[async_trait]
impl TonClientInterface for TonClient {
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
    callback: Arc<dyn TonConnectionCallback>,
    conn: Mutex<Option<TonConnection>>,
    archive_nodes_only: bool,
}

impl PoolConnection {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError> {
        let mut guard = self.conn.lock().await;
        match guard.deref() {
            Some(conn) => Ok(conn.clone()),
            None => {
                let conn = if self.archive_nodes_only {
                    // connect to other node until it will be able to fetch the very first block
                    TonConnection::connect_to_archive(&self.params, self.callback.clone()).await?
                } else {
                    TonConnection::connect(&self.params, self.callback.clone()).await?
                };
                *guard = Some(conn.clone());
                Ok(conn)
            }
        }
    }
}
