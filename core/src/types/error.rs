use thiserror::Error;

#[derive(Error, Debug)]
#[error("Invalid address (Address: {address}, message: {message})")]
pub struct TonAddressParseError {
    address: String,
    message: String,
}

impl TonAddressParseError {
    pub fn new<A: ToString, M: ToString>(address: A, message: M) -> TonAddressParseError {
        TonAddressParseError {
            address: address.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Invalid TransactionId (TxId: {txid}, message: {message})")]
pub struct TransactionIdParseError {
    txid: String,
    message: String,
}

impl TransactionIdParseError {
    pub fn new<T: ToString, M: ToString>(txid: T, message: M) -> TransactionIdParseError {
        TransactionIdParseError {
            txid: txid.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Invalid TonHash (Hash: {hash}, message: {message})")]
pub struct TonHashParseError {
    hash: String,
    message: String,
}

impl TonHashParseError {
    pub fn new<T: ToString, M: ToString>(hash: T, message: M) -> TonHashParseError {
        TonHashParseError {
            hash: hash.to_string(),
            message: message.to_string(),
        }
    }
}

impl From<TonHashParseError> for TonAddressParseError {
    fn from(error: TonHashParseError) -> Self {
        TonAddressParseError {
            address: error.hash,
            message: error.message,
        }
    }
}
