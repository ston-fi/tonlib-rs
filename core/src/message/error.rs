use core::fmt;

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

    #[error("Invalid message ({0})")]
    InvalidMessage(InvalidMessage),
}

#[derive(Clone, PartialEq)]
pub struct InvalidMessage {
    pub opcode: Option<u32>,
    pub query_id: Option<u64>,
    pub message: String,
}

impl fmt::Display for InvalidMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InvalidMessage {{ opcode: {:?}, query_id: {:?}, message: {} }}",
            self.opcode, self.query_id, self.message
        )
    }
}

impl std::fmt::Debug for InvalidMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvalidMessage")
            .field("opcode", &self.opcode.map(|op| format!("{:#x}", op)))
            .field("query_id", &self.query_id)
            .field("message", &self.message)
            .finish()
    }
}
