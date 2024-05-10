#[cfg(feature = "state_cache")]
use std::sync::Arc;

use thiserror::Error;

use crate::address::TonAddress;
use crate::cell::TonCellError;
use crate::client::TonClientError;
use crate::emulator::TvmEmulatorError;
use crate::tl::TvmStackError;
use crate::types::{StackParseError, TonMethodId, TvmStackEntry};

#[derive(Error, Debug)]
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

    #[error("Illegal argument ({0})")]
    IllegalArgument(String),

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
    TvmStackParseError {
        method: TonMethodId,
        address: TonAddress,
        error: StackParseError,
    },

    #[error(
        "Tvm run error (Method: {method}, address: {address}, exit code: {exit_code}, gas used: {gas_used}, stack: {stack:?}, vm_log: {vm_log:?}, missing_library: {missing_library:?})"
    )]
    TvmRunError {
        method: TonMethodId,
        address: TonAddress,
        vm_log: Option<String>,
        exit_code: i32,
        stack: Vec<TvmStackEntry>,
        missing_library: Option<String>,
        gas_used: i64,
    },

    // TODO: Experiment with it, maybe just use  `CacheError { message: String }`
    #[cfg(feature = "state_cache")]
    #[error("{0}")]
    CacheError(#[from] Arc<TonContractError>),
}

pub trait MapStackError<R> {
    fn map_stack_error(
        self,
        method: &'static str,
        address: &TonAddress,
    ) -> Result<R, TonContractError>;
}

pub trait MapCellError<R> {
    fn map_cell_error(
        self,
        method: &'static str,
        address: &TonAddress,
    ) -> Result<R, TonContractError>;
}

impl<R> MapStackError<R> for Result<R, TvmStackError> {
    fn map_stack_error(
        self,
        method: &'static str,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(|e| TonContractError::MethodResultStackError {
            method: method.into(),
            address: address.clone(),
            error: e,
        })
    }
}

impl<R> MapStackError<R> for Result<R, StackParseError> {
    fn map_stack_error(
        self,
        method: &'static str,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(|e| TonContractError::TvmStackParseError {
            method: method.into(),
            address: address.clone(),
            error: e,
        })
    }
}

impl<R> MapCellError<R> for Result<R, TonCellError> {
    fn map_cell_error(
        self,
        method: &'static str,
        address: &TonAddress,
    ) -> Result<R, TonContractError> {
        self.map_err(|e| TonContractError::MethodResultStackError {
            method: method.into(),
            address: address.clone(),
            error: e.into(),
        })
    }
}
