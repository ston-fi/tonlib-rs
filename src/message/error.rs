use thiserror::Error;

use crate::cell::TonCellError;

#[derive(Error, Debug)]
pub enum TonMessageError {
    #[error("nacl error: (message)")]
    NaclCryptographicError { message: String },

    #[error("Forward_ton_amount must be positive when specifying forward_payload")]
    ForwardTonAmountIsNegative,

    #[error("TonCellError: {cell_error}")]
    TonCellError {
        #[from]
        cell_error: TonCellError,
    },
}
