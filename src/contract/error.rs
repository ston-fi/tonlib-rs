#[cfg(feature = "state_cache")]
use std::sync::Arc;
use thiserror::Error;

use crate::address::TonAddress;
use crate::cell::TonCellError;
use crate::client::TonClientError;
use crate::tl::{TvmStackEntry, TvmStackError};

#[derive(Error, Debug)]
pub enum TonContractError {
    #[error(
        "Tvm run error: code: {exit_code}, method: {method}, gas: {gas_used}, stack: {stack:?}"
    )]
    TvmRunError {
        method: String,
        gas_used: i64,
        stack: Vec<TvmStackEntry>,
        exit_code: i32,
    },

    #[error("Method result stack error: '{method}', address: {address}, stack error: {error:?}")]
    MethodResultStackError {
        method: String,
        address: TonAddress,
        error: TvmStackError,
    },

    #[error("Cell error: '{method}', address: {address}, error {error}")]
    CellError {
        method: String,
        address: TonAddress,
        error: TonCellError,
    },

    #[error("{0}")]
    ClientError(#[from] TonClientError),

    #[error("Invalid method result stack: '{method}', address: {address}, actual: {actual}, expected {expected}")]
    InvalidMethodResultStackSize {
        method: String,
        address: TonAddress,
        actual: usize,
        expected: usize,
    },

    #[error("Internal error: {message}")]
    InternalError { message: String },

    #[error("Illegal argument: {message}")]
    IllegalArgument { message: String },

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
            method: method.to_string(),
            address: address.clone(),
            error: e.into(),
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
            method: method.to_string(),
            address: address.clone(),
            error: e.into(),
        })
    }
}
