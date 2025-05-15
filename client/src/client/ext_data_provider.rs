use async_trait::async_trait;

use crate::client::TonClientError;
use crate::tl::{TonFunction, TonResult};

/// Allows to intercept TonFunction calls to LiteNode
#[async_trait]
pub trait ExternalDataProvider: Send + Sync {
    async fn handle(&self, function: &TonFunction) -> Option<Result<TonResult, TonClientError>>;
}
