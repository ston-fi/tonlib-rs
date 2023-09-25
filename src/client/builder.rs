use crate::client::{
    DefaultConnectionCallback, RetryStrategy, TonClient, TonConnectionCallback, TonConnectionParams,
};

use crate::client::error;
use std::sync::Arc;

pub struct TonClientBuilder {
    pool_size: usize,
    connection_params: TonConnectionParams,
    retry_strategy: RetryStrategy,
    callback: Arc<dyn TonConnectionCallback + Send + Sync>,
}

impl TonClientBuilder {
    pub fn new() -> Self {
        TonClientBuilder {
            pool_size: 1,
            connection_params: TonConnectionParams::default(),
            retry_strategy: RetryStrategy::default(),
            callback: Arc::new(DefaultConnectionCallback {}),
        }
    }

    pub fn with_pool_size(&mut self, pool_size: usize) -> &mut Self {
        self.pool_size = pool_size;
        self
    }

    pub fn with_connection_params(&mut self, connection_params: &TonConnectionParams) -> &mut Self {
        self.connection_params = connection_params.clone();
        self
    }

    pub fn with_retry_strategy(&mut self, retry_strategy: &RetryStrategy) -> &mut Self {
        self.retry_strategy = retry_strategy.clone();
        self
    }

    pub fn with_callback(
        &mut self,
        callback: Arc<dyn TonConnectionCallback + Send + Sync>,
    ) -> &mut Self {
        self.callback = callback;
        self
    }

    pub fn with_default_callback(&mut self) -> &mut Self {
        self.callback = Arc::new(DefaultConnectionCallback {});
        self
    }

    pub fn with_keystore_dir(&mut self, keystore_dir: String) -> &mut Self {
        self.connection_params.keystore_dir = Some(keystore_dir);
        self
    }

    pub fn without_keystore(&mut self) -> &mut Self {
        self.connection_params.keystore_dir = None;
        self
    }

    pub async fn build(&self) -> Result<TonClient, error::TonClientError> {
        TonClient::new(
            self.pool_size,
            &self.connection_params,
            &self.retry_strategy,
            self.callback.clone(),
        )
        .await
    }
}
