use reqwest::StatusCode;
use thiserror::Error;

use crate::meta::{IpfsLoaderError, MetaDataContent};

#[derive(Debug, Error)]
pub enum MetaLoaderError {
    #[error("Unsupported content layout (Metadata content: {0:?})")]
    ContentLayoutUnsupported(MetaDataContent),

    #[error("Failed to load jetton metadata (URI: {uri}, response status code: {status})")]
    LoadMetaDataFailed { uri: String, status: StatusCode },

    #[error("IpfsLoaderError ({0})")]
    IpfsLoaderError(#[from] IpfsLoaderError),

    #[error("Serde_json Error ({0})")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Transport error ({0})")]
    TransportError(#[from] reqwest::Error),
}
