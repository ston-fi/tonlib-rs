use std::string::FromUtf8Error;

use thiserror::Error;
use tonlib_core::cell::TonCellError;

use crate::types::TvmStackEntry;

#[derive(Error, Debug)]
pub enum StackParseError {
    #[error("Invalid stack entry type{{expected: {expected}, found: {found}}}")]
    InvalidEntryType {
        expected: String,
        found: TvmStackEntry,
    },

    #[error("Invalid stack size({0})")]
    InvalidStackSize(usize),

    #[error("Invalid stack entry({0})")]
    InvalidEntryValue(String),

    #[error("FromUtf8 error({0})")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("Cell error({0})")]
    CellError(#[from] TonCellError),
}
