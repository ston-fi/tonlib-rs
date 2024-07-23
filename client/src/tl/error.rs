use std::ffi::NulError;
use std::str::Utf8Error;

use thiserror::Error;
use tonlib_core::cell::TonCellError;

use crate::tl::stack::TvmStackEntry;

#[derive(Error, Debug)]
pub enum TvmStackError {
    #[error("Unsupported conversion to string (TvmStackEntry: {e}, index: {index})")]
    StringConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to i32 (TvmStackEntry: {e}, index: {index})")]
    I32Conversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to i64 (TvmStackEntry: {e}, index: {index})")]
    I64Conversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BigUint (TvmStackEntry: {e}, index: {index})")]
    BigUintConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BigInt (TvmStackEntry: {e}, index: {index})")]
    BigIntConversion { e: TvmStackEntry, index: usize },

    #[error("Unsupported conversion to BagOfCells (TvmStackEntry: {e}, index: {index})")]
    BoCConversion { e: TvmStackEntry, index: usize },

    #[error("Invalid tvm stack index ( Index: {index}, total length {len})")]
    InvalidTvmStackIndex { index: usize, len: usize },

    #[error("TonCellError ({0})")]
    TonCellError(#[from] TonCellError),
}

#[derive(Error, Debug)]
pub enum TlError {
    #[error("Utf8 Error ({0})")]
    Utf8Error(#[from] Utf8Error),

    #[error("Serde_json Error ({0})")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("CString is null ({0})")]
    NulError(#[from] NulError),
}
