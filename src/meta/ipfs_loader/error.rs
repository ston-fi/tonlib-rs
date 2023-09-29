use std::string::FromUtf8Error;

use reqwest::StatusCode;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum IpfsLoaderError {
    #[error("Failed to load IPFS object {path}, status: {status}, message: {message}")]
    IpfsLoadObjectFailed {
        path: String,
        status: StatusCode,
        message: String,
    },

    #[error("Transport error: {0}")]
    TransportError(#[from] reqwest::Error),

    #[error("FromUtf8 Error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
}
