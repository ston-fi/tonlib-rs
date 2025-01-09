use std::borrow::Cow;
use std::sync::Arc;

use thiserror::Error;
use tonlib_core::cell::TonCellError;
use tonlib_core::types::TonHashParseError;
use tonlib_core::TonAddress;

use crate::client::TonClientError;
use crate::emulator::error::TvmEmulatorError;
use crate::tl::TvmStackError;
use crate::types::{StackParseError, TonMethodId, TvmStackEntry};

#[derive(Error, Debug)]
#[allow(clippy::result_large_err)]
pub enum TonContractError {
    #[error("Cell error (Method: {method}, address: {address}, error {error}")]
    CellError {
        method: String,
        address: TonAddress,
        error: TonCellError,
    },
    #[error("TonClientError ({0})")]
    ClientError(#[from] TonClientError),

    #[error("Method emulation error (Method: {method}, address: {address}, error {error}")]
    MethodEmulationError {
        method: String,
        address: TonAddress,
        error: TvmEmulatorError,
    },

    #[error("Message emulation error (address: {address}, error {error}")]
    MessageEmulationError {
        address: TonAddress,
        error: TvmEmulatorError,
    },

    #[error("Invalid argument ({0})")]
    InvalidArgument(String),

    #[error("Internal error ({0})")]
    InternalError(String),

    #[error("Invalid method result stack size  (Method: {method}, address: {address}, actual: {actual}, expected {expected})")]
    InvalidMethodResultStackSize {
        method: String,
        address: TonAddress,
        actual: usize,
        expected: usize,
    },

    #[error(
        "Method result stack error (Method: {method}, address: {address}, stack error: {error:?})"
    )]
    MethodResultStackError {
        method: TonMethodId,
        address: TonAddress,
        error: TvmStackError,
    },

    #[error("Missing library (Method: {method}, address: {address}, lib: {missing_library})")]
    MissingLibrary {
        method: TonMethodId,
        address: TonAddress,
        missing_library: String,
    },

    #[error("Library not found (Address: {address}, lib: {missing_library})")]
    LibraryNotFound {
        address: TonAddress,
        missing_library: String,
    },

    #[error(
        "Tvm stack parse  error (Method: {method}, address: {address}, stack error: {error:?})"
    )]
    #[allow(clippy::result_large_err)]
    TvmStackParseError {
        method: TonMethodId,
        address: TonAddress,
        error: Box<StackParseError>,
    },

    #[error(
        "Tvm run error (Method: {method}, address: {address}, exit code: {exit_code}, gas used: {gas_used}, stack: {stack:?}, vm_log: {vm_log:?}, missing_library: {missing_library:?})"
    )]
    #[allow(clippy::result_large_err)]
    TvmRunError {
        method: TonMethodId,
        address: TonAddress,
        vm_log: Box<Option<String>>,
        exit_code: i32,
        stack: Box<Vec<TvmStackEntry>>,
        missing_library: Option<String>,
        gas_used: i64,
    },

    #[error("{0}")]
    CacheError(#[from] Arc<TonContractError>),

    #[error("{0}")]
    TonLibraryError(#[from] TonLibraryError),
}

pub trait MapStackError<R> {
    #[allow(clippy::result_large_err)]
    fn map_stack_error(
        self,
        method: impl Into<Cow<'static, str>>,
        address: &TonAddress,
    ) -> Result<R, TonContractError>;
}

pub trait MapCellError<R> {
    #[allow(clippy::result_large_err)]
    fn map_cell_error(
        self,
        method: impl Into<Cow<'static, str>>,
        address: &TonAddress,
    ) -> Result<R, TonContractError>;
}

impl<R> MapStackError<R> for Result<R, TvmStackError> {
    fn map_stack_error(
        self,
        method: impl Into<Cow<'static, str>>,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(
            |e: TvmStackError| TonContractError::MethodResultStackError {
                method: TonMethodId::Name(method.into()),
                address: address.clone(),
                error: e,
            },
        )
    }
}

impl<R> MapStackError<R> for Result<R, StackParseError> {
    fn map_stack_error(
        self,
        method: impl Into<Cow<'static, str>>,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(|e| TonContractError::TvmStackParseError {
            method: TonMethodId::Name(method.into()),
            address: address.clone(),
            error: e.into(),
        })
    }
}

impl<R> MapCellError<R> for Result<R, TonCellError> {
    fn map_cell_error(
        self,
        method: impl Into<Cow<'static, str>>,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(|e| TonContractError::MethodResultStackError {
            method: TonMethodId::Name(method.into()),
            address: address.clone(),
            error: e.into(),
        })
    }
}

#[derive(Error, Debug)]
pub enum TonLibraryError {
    #[error("{0}")]
    TonClientError(#[from] TonClientError),

    #[error("{0}")]
    TonCellError(#[from] TonCellError),

    #[error("{0}")]
    TonHashParseError(#[from] TonHashParseError),

    #[error("Library not found for {0}")]
    LibraryNotFound(String),

    #[error("Expected exactly one library, but got multiple")]
    MultipleLibrariesReturned,

    #[error("Getting library by mc_seqno is not supported")]
    SeqnoNotSupported,
}
