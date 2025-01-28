mod address;
mod error;
mod tx_id;

use std::fmt;

pub use address::*;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
pub use error::*;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
pub use tx_id::*;

pub const TON_HASH_LEN: usize = 32;

pub const ZERO_HASH: TonHash = TonHash([0u8; TON_HASH_LEN]);

pub const DEFAULT_CELL_HASH: TonHash = TonHash([
    150, 162, 150, 210, 36, 242, 133, 198, 123, 238, 147, 195, 15, 138, 48, 145, 87, 240, 218, 163,
    93, 197, 184, 126, 65, 11, 120, 99, 10, 9, 207, 199,
]);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TonHash([u8; TON_HASH_LEN]);

impl TonHash {
    pub fn iter(&self) -> std::slice::Iter<'_, u8> {
        self.0.iter()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Convert the hash to a hexadecimal string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.as_slice())
    }

    /// Convert the hash to a Base64 string
    pub fn to_base64(&self) -> String {
        BASE64_URL_SAFE_NO_PAD.encode(self.0.as_slice())
    }

    /// Create a `TonHash` from a hexadecimal string
    pub fn from_hex(hex_str: &str) -> Result<Self, TonHashParseError> {
        let bytes = hex::decode(hex_str).map_err(|_| {
            TonHashParseError::new(hex_str, "Failed to convert hex string to TonHash")
        })?;
        Self::try_from(bytes)
    }

    /// Create a `TonHash` from a Base64 string
    pub fn from_base64(base64_str: &str) -> Result<Self, TonHashParseError> {
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(base64_str).map_err(|_| {
            TonHashParseError::new(base64_str, "Failed to convert base64 string to TonHash")
        })?;
        Self::try_from(bytes)
    }
}

impl From<[u8; TON_HASH_LEN]> for TonHash {
    fn from(arr: [u8; TON_HASH_LEN]) -> Self {
        TonHash(arr)
    }
}

impl From<TonHash> for [u8; TON_HASH_LEN] {
    fn from(arr: TonHash) -> [u8; TON_HASH_LEN] {
        arr.0
    }
}

impl From<&[u8; 32]> for TonHash {
    fn from(slice: &[u8; 32]) -> Self {
        TonHash(*slice)
    }
}

impl From<TonHash> for BigUint {
    fn from(value: TonHash) -> Self {
        BigUint::from_bytes_be(value.as_slice())
    }
}

impl TryFrom<&[u8]> for TonHash {
    type Error = TonHashParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != TON_HASH_LEN {
            let formatted_input = format!("{:?}", value);
            Err(TonHashParseError::new(
                formatted_input,
                format!(
                    "TonHash must contain {TON_HASH_LEN} bytes, but {} given",
                    value.len()
                ),
            ))
        } else {
            let mut hash = [0u8; TON_HASH_LEN];
            hash.copy_from_slice(value);
            Ok(TonHash(hash))
        }
    }
}

impl TryFrom<Vec<u8>> for TonHash {
    type Error = TonHashParseError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl fmt::Debug for TonHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to Display for Debug formatting
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for TonHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
