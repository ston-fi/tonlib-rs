use std::ffi::NulError;
use std::str::Utf8Error;
use thiserror::Error;

use crate::cell::TonCellError;
use crate::tl::stack::TvmStackEntry;

#[derive(Error, Debug)]
pub enum TvmStackError {
    #[error("Unsupported conversion to string from {e}, index: {index}")]
    StringConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to i32 from {e}, index: {index}")]
    I32Conversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to i64 from {e}, index: {index}")]
    I64Conversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BigUint from {e}, index: {index}")]
    BigUintConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BigInt from {e}, index: {index}")]
    BigIntConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BagOfCells from {e}, index: {index}")]
    BoCConversion { e: TvmStackEntry, index: usize },

    #[error("Invalid tvm stack index {index}, total length {len}")]
    InvalidTvmStackIndex { index: usize, len: usize },

    #[error("TonCellError: {0}")]
    TonCellError(#[from] TonCellError),
}

#[derive(Error, Debug)]
pub enum TlError {
    #[error("Utf8 Error: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("Serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("CString is null: {0}")]
    NulError(#[from] NulError),
}

#[derive(Error, Debug)]
#[error("Invalid TransactionId {txid}: {message}")]
pub struct InternalTransactionIdParseError {
    txid: String,
    message: String,
}

impl InternalTransactionIdParseError {
    pub fn new<T: ToString, M: ToString>(txid: T, message: M) -> InternalTransactionIdParseError {
        InternalTransactionIdParseError {
            txid: txid.to_string(),
            message: message.to_string(),
        }
    }
}
