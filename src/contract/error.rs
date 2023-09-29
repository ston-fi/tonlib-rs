use thiserror::Error;

use crate::{
    address::TonAddress,
    cell::TonCellError,
    client::TonClientError,
    tl::{TvmStackEntry, TvmStackError},
};

#[derive(Error, Debug)]
pub enum TonContractError {
    #[error("Tvm run error: code: {exit_code}, gas: {gas_used}, stack: {stack:?}")]
    TvmRunError {
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

    #[error("Ton client error: '{method}', address: {address}, error: {error} ")]
    ClientMethodError {
        method: String,
        address: String,
        error: TonClientError,
    },

    #[error("Invalid method result stack: '{method}', address: {address}, actual: {actual}, expected {expected}")]
    InvalidMethodResultStackSize {
        method: String,
        address: TonAddress,
        actual: usize,
        expected: usize,
    },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

pub trait MapStackError<R> {
    fn map_stack_error<T>(self, method: T, address: &TonAddress) -> Result<R, TonContractError>
    where
        T: ToString;
}

pub trait MapCellError<R> {
    fn map_cell_error<T>(self, method: T, address: &TonAddress) -> Result<R, TonContractError>
    where
        T: ToString;
}

impl TonContractError {
    pub fn client_method_error<T>(
        method: T,
        address: Option<&TonAddress>,
        error: TonClientError,
    ) -> TonContractError
    where
        T: ToString,
    {
        TonContractError::ClientMethodError {
            method: method.to_string(),
            address: if let Some(addr) = address {
                addr.to_string()
            } else {
                "N/A".to_string()
            },
            error: error,
        }
    }
}

impl<R> MapStackError<R> for Result<R, TvmStackError> {
    fn map_stack_error<T>(self, method: T, address: &TonAddress) -> Result<R, TonContractError>
    where
        T: ToString,
    {
        self.map_err(|e| TonContractError::MethodResultStackError {
            method: method.to_string(),
            address: address.clone(),
            error: e.into(),
        })
    }
}
impl<R> MapCellError<R> for Result<R, TonCellError> {
    fn map_cell_error<T>(self, method: T, address: &TonAddress) -> Result<R, TonContractError>
    where
        T: ToString,
    {
        self.map_err(|e| TonContractError::MethodResultStackError {
            method: method.to_string(),
            address: address.clone(),
            error: e.into(),
        })
    }
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Limit ({limit}) must not exceed capacity ({capacity})")]
    LimitExceeded { limit: usize, capacity: usize },

    #[error("ContractError: {contract_error}")]
    ContractError {
        #[from]
        contract_error: TonContractError,
    },
}
