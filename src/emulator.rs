use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub use error::*;
use num_bigint::Sign;
pub use unsafe_emulator::*;

use self::types::TvmEmulatorMessageResponse;
use crate::address::TonAddress;
use crate::cell::{BagOfCells, Cell, CellBuilder};
use crate::emulator::types::TvmEmulatorResponse;
use crate::types::{TonMethodId, TvmMsgSuccess, TvmStackEntry, TvmSuccess};

mod error;
mod types;
mod unsafe_emulator;

#[derive(Debug)]
pub struct TvmEmulator {
    emulator: TvmEmulatorUnsafe,
}

const DEFAULT_VM_LOG_VERBOSITY: u32 = 1;

pub struct TvmEmulatorC7Builder<'a> {
    pub address: &'a TonAddress,
    pub config: &'a [u8],
    pub balance: u64,
    pub unix_time: u64,
    pub seed: [u8; 32],
}

#[derive(Clone)]
pub struct TvmEmulatorC7 {
    pub address: TonAddress,
    pub config: Vec<u8>,
    pub balance: u64,
    pub unix_time: u64,
    pub seed: [u8; 32],
}

impl<'a> TvmEmulatorC7Builder<'a> {
    pub fn new(address: &'a TonAddress, config: &'a [u8], balance: u64) -> Self {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards!")
            .as_secs();

        TvmEmulatorC7Builder {
            address,
            config,
            balance,
            unix_time,
            seed: [0; 32],
        }
    }

    pub fn with_seed(&mut self, seed: [u8; 32]) -> &mut Self {
        self.seed = seed;
        self
    }

    pub fn with_unix_time(&mut self, unix_time: u64) -> &mut Self {
        self.unix_time = unix_time;
        self
    }

    pub fn build(&self) -> TvmEmulatorC7 {
        TvmEmulatorC7 {
            address: self.address.clone(),
            config: self.config.to_vec(),
            balance: self.balance,
            unix_time: self.unix_time,
            seed: self.seed,
        }
    }
}

impl TvmEmulator {
    pub fn new(code: &[u8], data: &[u8]) -> Result<TvmEmulator, TvmEmulatorError> {
        let emulator = TvmEmulatorUnsafe::create(code, data, DEFAULT_VM_LOG_VERBOSITY)?;
        let ton_contract_emulator = TvmEmulator { emulator };
        Ok(ton_contract_emulator)
    }

    pub fn set_c7(&mut self, c7: &TvmEmulatorC7) -> Result<&mut Self, TvmEmulatorError> {
        let addr_str = c7.address.to_hex();
        let hex_str = hex::encode(c7.seed);
        let seed = hex_str.as_bytes();
        let config = c7.config.as_slice();
        let unix_time = c7.unix_time as u32;
        let balance = c7.balance;
        let res = self
            .emulator
            .set_c7(addr_str.as_bytes(), unix_time, balance, seed, config)?;
        if res {
            Ok(self)
        } else {
            Err(TvmEmulatorError::EmulatorError(
                "Couldn't set c7".to_string(),
            ))
        }
    }

    pub fn set_debug_enable(&mut self) -> Result<&mut Self, TvmEmulatorError> {
        let result = self.emulator.set_debug_enabled(true);
        match result {
            true => Ok(self),
            false => Err(TvmEmulatorError::InternalError(
                "Unable to set debug enable".to_string(),
            )),
        }
    }

    pub fn set_gas_limit(&mut self, gas_limit: u64) -> Result<&mut Self, TvmEmulatorError> {
        let result = self.emulator.set_gas_limit(gas_limit);
        match result {
            true => Ok(self),
            false => Err(TvmEmulatorError::InternalError(
                "Unable to set gas limit".to_string(),
            )),
        }
    }

    pub fn set_libraries(&mut self, libraries: &[u8]) -> Result<&mut Self, TvmEmulatorError> {
        if libraries.is_empty() {
            return Ok(self);
        }
        let res = self.emulator.set_libraries(libraries)?;
        if res {
            Ok(self)
        } else {
            Err(TvmEmulatorError::EmulatorError(
                "Couldn't set libraries".to_string(),
            ))
        }
    }

    pub fn send_internal_message(
        &mut self,
        msg: Cell,
        amount: u64,
    ) -> Result<TvmMsgSuccess, TvmEmulatorError> {
        let msg_serialized = BagOfCells::from_root(msg).serialize(false)?;
        let msg_result = self
            .emulator
            .send_internal_message(msg_serialized.as_slice(), amount)?;
        let response = TvmEmulatorMessageResponse::from_json(msg_result.as_str());
        response
    }

    pub fn send_external_message(&mut self, msg: Cell) -> Result<TvmMsgSuccess, TvmEmulatorError> {
        let msg_serialized = BagOfCells::from_root(msg).serialize(false)?;
        let msg_result = self
            .emulator
            .send_external_message(msg_serialized.as_slice())?;
        let response = TvmEmulatorMessageResponse::from_json(msg_result.as_str());
        response
    }

    pub fn run_get_method(
        &mut self,
        method: &TonMethodId,
        stack: &[TvmStackEntry],
    ) -> Result<TvmSuccess, TvmEmulatorError> {
        let stack_boc = Self::build_stack_boc(stack)?;
        let run_result = self.emulator.run_get_method(method.to_id(), &stack_boc)?;
        let response = TvmEmulatorResponse::from_json(run_result.as_str())?;

        Ok(response)
    }

    #[allow(clippy::let_and_return)]
    fn build_stack_boc(stack: &[TvmStackEntry]) -> Result<Vec<u8>, TvmEmulatorError> {
        let root_cell = if stack.is_empty() {
            // empty stack should contain header cell with 24 bit number containing number of elements (0)
            // and reference to empty cell
            // Cell{ data: [000000], bit_len: 24, references: [
            //     Cell{ data: [], bit_len: 0, references: [
            //     ] }
            // ] }
            let empty_cell = CellBuilder::new().build()?;
            let root_cell = CellBuilder::new()
                .store_u64(24, 0)?
                .store_reference(&Arc::new(empty_cell))?
                .build()?;
            root_cell
        } else {
            let mut prev_cell: Cell = CellBuilder::new().build()?;
            for i in 0..stack.len() {
                let mut builder = CellBuilder::new();
                builder.store_child(prev_cell)?;
                if i == stack.len() - 1 {
                    builder.store_u32(24, stack.len() as u32)?;
                }
                Self::store_stack_entry(&mut builder, &stack[i])?;
                let new_cell = builder.build()?;
                prev_cell = new_cell;
            }
            prev_cell
        };
        log::trace!("Produced stack:\n{:?}", root_cell);
        Ok(BagOfCells::from_root(root_cell).serialize(false)?)
    }

    fn store_stack_entry(
        builder: &mut CellBuilder,
        entry: &TvmStackEntry,
    ) -> Result<(), TvmEmulatorError> {
        match entry {
            TvmStackEntry::Null => {
                builder.store_byte(0)?;
                Ok(())
            }
            TvmStackEntry::Nan => {
                builder.store_byte(2)?.store_byte(0xff)?;
                Ok(())
            }
            TvmStackEntry::Int64(val) => {
                builder.store_byte(1)?.store_i64(64, *val)?;
                Ok(())
            }
            TvmStackEntry::Int257(val) => {
                let (sign, mag) = val.clone().into_parts();
                builder.store_byte(2)?;
                if sign == Sign::Minus {
                    builder.store_byte(1)?;
                } else {
                    builder.store_byte(0)?;
                };
                builder.store_uint(256, &mag)?;
                Ok(())
            }
            TvmStackEntry::Cell(cell) => {
                builder.store_reference(cell)?;
                builder.store_byte(3)?;
                Ok(())
            }
            TvmStackEntry::Slice(slice) => {
                builder.store_reference(&slice.cell)?;
                builder.store_byte(4)?;
                builder.store_u32(10, slice.start_bit as u32)?; // st_bits
                builder.store_u32(10, slice.end_bit as u32)?; // en_bits
                builder.store_u8(3, slice.start_ref as u8)?; // st_ref
                builder.store_u8(3, slice.end_ref as u8)?; // en_ref
                Ok(())
            }
            TvmStackEntry::Unsupported => Err(TvmEmulatorError::EmulatorError(
                "EmulatorStackEntry::Unsupported is not supported".to_string(),
            )),
        }
    }
}
