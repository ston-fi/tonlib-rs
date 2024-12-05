use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use base64::alphabet::{STANDARD, URL_SAFE};
use base64::engine::general_purpose::{NO_PAD, PAD};
use base64::engine::GeneralPurpose;
use base64::Engine;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::{TonHash, TransactionIdParseError};
use crate::types::ZERO_HASH;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TonTxId {
    pub lt: i64,
    pub hash: TonHash,
}

lazy_static! {
    pub static ref NULL_TRANSACTION_ID: TonTxId = TonTxId {
        lt: 0i64,
        hash: ZERO_HASH
    };
}

impl TonTxId {
    pub fn hash_string(&self) -> String {
        self.hash.to_hex()
    }

    pub fn to_formatted_string(&self) -> String {
        format!("{}:{}", self.lt, self.hash_string())
    }

    pub fn from_lt_hash(lt: i64, hash_str: &str) -> Result<TonTxId, TransactionIdParseError> {
        let hash: TonHash = if hash_str.len() == 64 {
            match hex::decode(hash_str) {
                Ok(hash) => Self::tx_hash_to_array(lt, hash)?,
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
                Ok(hash) => Self::tx_hash_to_array(lt, hash)?,
                Err(_) => {
                    return Err(TransactionIdParseError::new(
                        format!("{}, {}", lt, hash_str),
                        "Invalid transaction hash: base64 decode error",
                    ))
                }
            }
        };

        Ok(TonTxId { lt, hash })
    }

    fn tx_hash_to_array(lt: i64, hash: Vec<u8>) -> Result<TonHash, TransactionIdParseError> {
        if hash.len() == 32 {
            match &hash.clone().try_into() {
                Ok(array) => Ok(*array),
                Err(_) => Err(TransactionIdParseError::new(
                    format!("{}:{:?}", lt, hash),
                    "Incorrect tx hash format".to_string(),
                )),
            }
        } else {
            Err(TransactionIdParseError::new(
                format!("{}:{:?}", lt, hash),
                "Incorrect tx hash format".to_string(),
            ))
        }
    }
}

impl FromStr for TonTxId {
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
        TonTxId::from_lt_hash(lt, hash_str)
    }
}

impl Display for TonTxId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_formatted_string().as_str())
    }
}

impl Debug for TonTxId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_formatted_string().as_str())
    }
}
