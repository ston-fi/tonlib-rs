use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
pub use block_functions::*;
pub use block_stream::*;
pub use builder::*;
pub use callback::*;
pub use connection::*;
pub use error::*;
pub use interface::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::RetryIf;
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

/// Check on perform upon connection
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionCheck {
    /// No check.
    None,
    /// Verify node aliveness
    Health,
    /// Verify that connected to archive node
    Archive,
}

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
        connection_check: ConnectionCheck,
    ) -> Result<TonClient, TonClientError> {
        let mut connections = Vec::with_capacity(pool_size);
        for i in 0..pool_size {
            let mut p = params.clone();
            if let Some(dir) = &params.keystore_dir {
                let keystore_prefix = Path::new(dir.as_str());
                let keystore_dir = keystore_prefix.join(format!("{}", i));
                fs::create_dir_all(&keystore_dir)?;
                let path_str = keystore_dir.into_os_string().into_string().map_err(|_| {
                    TonClientError::InternalError("Error constructing keystore path".to_string())
                })?;
                p.keystore_dir = Some(path_str)
            };
            let entry = PoolConnection {
                params: p,
                callback: callback.clone(),
                conn: Mutex::new(None),
                connection_check: connection_check.clone(),
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
        TonClientBuilder::default()
    }

    pub async fn default() -> Result<TonClient, TonClientError> {
        Self::builder().build().await
    }

    #[allow(clippy::let_and_return)]
    async fn retrying_invoke(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        let fi = FixedInterval::from_millis(self.inner.retry_strategy.interval_ms);
        let strategy = fi.take(self.inner.retry_strategy.max_retries);
        let result = RetryIf::spawn(strategy, || self.do_invoke(function), retry_condition).await;
        result
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

    #[allow(clippy::let_and_return)]
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
    connection_check: ConnectionCheck,
}

impl PoolConnection {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError> {
        let mut guard = self.conn.lock().await;
        match guard.deref() {
            Some(conn) => Ok(conn.clone()),
            None => {
                let conn = match self.connection_check {
                    ConnectionCheck::None => {
                        TonConnection::connect(&self.params, self.callback.clone()).await?
                    }
                    ConnectionCheck::Health => {
                        TonConnection::connect_healthy(&self.params, self.callback.clone()).await?
                    }
                    ConnectionCheck::Archive => {
                        TonConnection::connect_archive(&self.params, self.callback.clone()).await?
                    }
                };
                *guard = Some(conn.clone());
                Ok(conn)
            }
        }
    }
}
