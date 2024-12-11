use std::time::{SystemTime, UNIX_EPOCH};

use tonlib_core::types::ZERO_HASH;
use tonlib_core::{TonAddress, TonHash};

use crate::contract::TonContractError;

#[derive(Clone, Debug)]
pub struct TvmEmulatorC7 {
    pub address: TonAddress,
    pub config: Vec<u8>,
    pub balance: u64,
    pub unix_time: u64,
    pub seed: TonHash,
}

impl TvmEmulatorC7 {
    pub fn new(address: TonAddress, config: Vec<u8>) -> Result<Self, TonContractError> {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TonContractError::InternalError("time went back".to_string()))?
            .as_secs();

        let c7 = Self {
            address,
            config,
            balance: 0,
            unix_time,
            seed: ZERO_HASH,
        };
        Ok(c7)
    }

    pub fn with_balance(mut self, balance: u64) -> Self {
        self.balance = balance;
        self
    }

    pub fn with_seed(mut self, seed: TonHash) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_unix_time(mut self, unix_time: u64) -> Self {
        self.unix_time = unix_time;
        self
    }
}
