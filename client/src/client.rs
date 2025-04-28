use std::fs;
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
use tokio_retry::strategy::FixedInterval;
use tokio_retry::RetryIf;
pub use types::*;

use crate::client::ext_data_provider::ExternalDataProvider;
use crate::tl::*;

mod block_functions;
mod block_stream;
mod builder;
mod callback;
mod connection;
mod error;
mod interface;
mod types;

pub mod ext_data_provider;
#[cfg(feature = "liteapi")]
mod recent_init_block;

#[derive(Clone)]
pub struct TonClient {
    inner: Arc<Inner>,
}

struct Inner {
    retry_strategy: RetryStrategy,
    connections: Vec<TonConnection>,
}

impl TonClient {
    /// Creates a new TonClient
    pub async fn new(
        pool_size: usize,
        params: &TonConnectionParams,
        retry_strategy: RetryStrategy,
        callback: Arc<dyn TonConnectionCallback>,
        connection_check: ConnectionCheck,
        external_data_provider: Option<Arc<dyn ExternalDataProvider>>,
    ) -> Result<TonClient, TonClientError> {
        let patched_params = if params.update_init_block {
            patch_init_block(params).await?
        } else {
            params.clone()
        };
        let mut connections = Vec::with_capacity(pool_size);
        for i in 0..pool_size {
            let mut conn_params = patched_params.clone();
            if let Some(dir) = &patched_params.keystore_dir {
                let keystore_prefix = Path::new(dir.as_str());
                let keystore_dir = keystore_prefix.join(format!("{}", i));
                fs::create_dir_all(&keystore_dir)?;
                let path_str = keystore_dir.into_os_string().into_string().map_err(|_| {
                    TonClientError::InternalError("Error constructing keystore path".to_string())
                })?;
                conn_params.keystore_dir = Some(path_str)
            };
            let conn = TonConnection::new(
                connection_check.clone(),
                &conn_params,
                callback.clone(),
                external_data_provider.clone(),
            )
            .await?;

            connections.push(conn);
        }
        let inner = Inner {
            retry_strategy,
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
    fn random_item(&self) -> &TonConnection {
        let pos = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..self.inner.connections.len())
        };
        &self.inner.connections[pos]
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

#[cfg(not(feature = "liteapi"))]
async fn patch_init_block(
    params: &TonConnectionParams,
) -> Result<TonConnectionParams, TonClientError> {
    log::warn!("Feature 'liteapi' is disabled, patch_init_block does nothing");
    Ok(params.clone())
}

#[cfg(feature = "liteapi")]
async fn patch_init_block(
    params: &TonConnectionParams,
) -> Result<TonConnectionParams, TonClientError> {
    use crate::config::TonConfig;

    let mut ton_config = TonConfig::from_json(&params.config).map_err(|e| {
        let msg = format!("Fail to parse config: {}", e);
        TonClientError::InternalError(msg)
    })?;

    let recent_init_block = match recent_init_block::get_recent_init_block(&ton_config.liteservers)
        .await
    {
        Some(block) => block,
        None => {
            let msg = "Failed to update init_block: update it manually in network_config.json (https://docs.ton.org/develop/howto/network-configs)";
            return Err(TonClientError::InternalError(msg.to_string()));
        }
    };

    let old_seqno = ton_config.get_init_block_seqno();
    if old_seqno < recent_init_block.seqno {
        ton_config.set_init_block(&recent_init_block).map_err(|e| {
            let msg = format!("Fail to serialize block_id: {}", e);
            TonClientError::InternalError(msg)
        })?;
        log::info!(
            "init_block updated: old_seqno={}, new_seqno={}",
            old_seqno,
            recent_init_block.seqno
        );
    } else {
        log::info!("Init block is up to date, seqno: {}", old_seqno);
    }

    let mut patched_params = params.clone();
    patched_params.config = ton_config.to_json().map_err(|e| {
        let msg = format!("Fail to serialize config: {}", e);
        TonClientError::InternalError(msg)
    })?;
    Ok(patched_params)
}
