use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use base64::alphabet::{STANDARD, URL_SAFE};
use base64::engine::general_purpose::{NO_PAD, PAD};
use base64::engine::GeneralPurpose;
use base64::Engine;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TransactionId {
    pub lt: i64,
    pub hash: Vec<u8>,
}

lazy_static! {
    pub static ref NULL_TRANSACTION_ID: TransactionId = TransactionId {
        lt: 0i64,
        hash: vec![0u8; 32]
    };
}

impl TransactionId {
    pub fn hash_string(&self) -> String {
        hex::encode(self.hash.as_slice())
    }

    pub fn to_formatted_string(&self) -> String {
        format!("{}:{}", self.lt, self.hash_string())
    }

    pub fn from_lt_hash(lt: i64, hash_str: &str) -> Result<TransactionId, TransactionIdParseError> {
        let hash = if hash_str.len() == 64 {
            match hex::decode(hash_str) {
                Ok(hash) => hash,
                Err(_) => {
                    return Err(TransactionIdParseError::new(
                        format!("{}, {}", lt, hash_str),
                        "Invalid transaction hash: base64 decode error",
                    ))
                }
            }
        } else {
            let char_set = if hash_str.contains('-') || hash_str.contains('_') {
                URL_SAFE
            } else {
                STANDARD
            };
            let pad = hash_str.len() == 44;

            let config = if pad { PAD } else { NO_PAD };

            let engine = GeneralPurpose::new(&char_set, config);

            match engine.decode(hash_str) {
                Ok(hash) => hash,
                Err(_) => {
                    return Err(TransactionIdParseError::new(
                        format!("{}, {}", lt, hash_str),
                        "Invalid transaction hash: base64 decode error",
                    ))
                }
            }
        };
        if hash.len() != 32 {
            return Err(TransactionIdParseError::new(
                format!("{}, {}", lt, hash_str),
                "Invalid transaction hash: length is not equal to 32",
            ));
        }

        Ok(TransactionId { lt, hash })
    }
}

impl FromStr for TransactionId {
    type Err = TransactionIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(TransactionIdParseError::new(
                s,
                "Invalid transaction hash: wrong format",
            ));
        }
        let lt: i64 = match parts[0].parse() {
            Ok(lt) => lt,
            Err(_) => {
                return Err(TransactionIdParseError::new(
                    s,
                    "Invalid transaction hash: wrong format",
                ))
            }
        };
        let hash_str = parts[1];
        TransactionId::from_lt_hash(lt, hash_str)
    }
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_formatted_string().as_str())
    }
}

impl Debug for TransactionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_formatted_string().as_str())
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
