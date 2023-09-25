use reqwest::StatusCode;
use thiserror::Error;

use crate::ipfs::IpfsLoaderError;

use super::MetaDataContent;

#[derive(Debug, Error)]
pub enum MetaLoaderError {
    #[error("Failed to load jetton metadata from {uri}. Resp status: {status}")]
    LoadMetaDataFailed { uri: String, status: StatusCode },

    #[error("Unsupported content layout {content:?}")]
    ContentLayoutUnsupported { content: MetaDataContent },

    #[error("Transport error: {0}")]
    TransportError(#[from] reqwest::Error),

    #[error("IpfsLoaderError: {0}")]
    IpfsLoaderError(#[from] IpfsLoaderError),

    #[error("Serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
}
