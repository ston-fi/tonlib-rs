use std::io;

use crate::tl::{TlError, TonResult, TonResultDiscriminants};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TonClientError {
    #[error("Unexpected TonResult: {actual}, expected: {expected}")]
    UnexpectedTonResult {
        actual: TonResultDiscriminants,
        expected: TonResultDiscriminants,
    },

    #[error("TonError: code {code}, message {message}")]
    TonlibError {
        method: &'static str,
        code: i32,
        message: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TlError: {0}")]
    TlError(#[from] TlError),

    #[error("Internal error: {message}")]
    InternalError { message: String },

    #[error("Illegal argument: {message}")]
    IllegalArgument { message: String },
}

impl TonClientError {
    pub fn unexpected_ton_result(
        expected: TonResultDiscriminants,
        actual: TonResult,
    ) -> TonClientError {
        TonClientError::UnexpectedTonResult {
            actual: actual.into(),
            expected,
        }
    }
}
