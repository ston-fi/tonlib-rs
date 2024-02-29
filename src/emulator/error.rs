use std::ffi::NulError;
use std::num::ParseIntError;
use std::str::Utf8Error;

use thiserror::Error;

use crate::cell::TonCellError;

#[derive(Error, Debug)]
pub enum TvmEmulatorError {
    #[error("Emulator creation failed")]
    CreationFailed(),

    #[error("Emulator error({0})")]
    EmulatorError(String),

    #[error("Internal error({0})")]
    InternalError(String),

    #[error("Cell error({0})")]
    CellError(#[from] TonCellError),

    #[error("Missing json field({0})")]
    MissingJsonField(&'static str),

    #[error("CString is null({0})")]
    NulError(#[from] NulError),

    #[error("ParseIntError( {0})")]
    ParseIntError(#[from] ParseIntError),

    #[error("Serde Json error({0})")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("FromUtf8 error({0})")]
    Utf8Error(#[from] Utf8Error),
}
