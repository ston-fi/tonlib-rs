use reqwest::StatusCode;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum IpfsLoaderError {
    #[error("Failed to load IPFS object (path: {path}, status: {status}, message: {message})")]
    IpfsLoadObjectFailed {
        path: String,
        status: StatusCode,
        message: String,
    },

    #[error("Transport error: {0}")]
    TransportError(#[from] reqwest::Error),
}
