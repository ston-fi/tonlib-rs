use crate::client::TonClientError;
use crate::tl::{TonFunction, TonResult};

/// Allows to intercept TonFunction calls to LiteNode
pub trait ExternalDataProvider: Send + Sync {
    fn handle(&self, function: &TonFunction) -> Option<Result<TonResult, TonClientError>>;
}
