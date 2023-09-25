use thiserror::Error;

use crate::contract::TonContractError;

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
