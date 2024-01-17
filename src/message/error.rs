use thiserror::Error;

use crate::cell::TonCellError;

#[derive(Error, Debug)]
pub enum TonMessageError {
    #[error("ForwardTonAmountIsNegative error: Forward_ton_amount must be positive when specifying forward_payload")]
    ForwardTonAmountIsNegative,

    #[error("NaCl cryptographic error ({0})")]
    NaclCryptographicError(String),

    #[error("TonCellError ({0})")]
    TonCellError(#[from] TonCellError),
}
