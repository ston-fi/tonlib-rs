use tonlib_core::cell::{BagOfCells, Cell};

use crate::emulator::c7_register::TvmEmulatorC7;
use crate::emulator::error::TvmEmulatorError;
use crate::emulator::tvm_emulator_unsafe::TvmEmulatorUnsafe;
use crate::emulator::types::{TvmEmulatorMessageResponse, TvmEmulatorResponse};
use crate::emulator::utils::build_stack_boc;
use crate::types::{TonMethodId, TvmMsgSuccess, TvmStackEntry, TvmSuccess};

#[derive(Debug)]
pub struct TvmEmulator {
    emulator: TvmEmulatorUnsafe,
}

const DEFAULT_VM_LOG_VERBOSITY: u32 = 1;

// construct part
impl TvmEmulator {
    pub fn new(code: &[u8], data: &[u8]) -> Result<TvmEmulator, TvmEmulatorError> {
        let emulator = TvmEmulatorUnsafe::new(code, data, DEFAULT_VM_LOG_VERBOSITY)?;
        let ton_contract_emulator = TvmEmulator { emulator };
        Ok(ton_contract_emulator)
    }

    pub fn with_c7(&mut self, c7: &TvmEmulatorC7) -> Result<&mut Self, TvmEmulatorError> {
        let addr_str = c7.address.to_hex();
        let hex_str = c7.seed.to_hex();
        let seed = hex_str.as_bytes();
        let config = c7.config.as_slice();
        let unix_time = c7.unix_time as u32;
        let balance = c7.balance;
        let res = self
            .emulator
            .set_c7(addr_str.as_bytes(), unix_time, balance, seed, config)?;

        if res {
            return Ok(self);
        }

        Err(TvmEmulatorError::EmulatorError(
            "Couldn't set c7".to_string(),
        ))
    }

    pub fn with_debug_enabled(&mut self) -> Result<&mut Self, TvmEmulatorError> {
        if self.emulator.set_debug_enabled(true) {
            return Ok(self);
        }
        Err(TvmEmulatorError::InternalError(
            "Unable to set debug enable".to_string(),
        ))
    }

    pub fn with_gas_limit(&mut self, gas_limit: u64) -> Result<&mut Self, TvmEmulatorError> {
        if self.emulator.set_gas_limit(gas_limit) {
            return Ok(self);
        }
        Err(TvmEmulatorError::InternalError(
            "Unable to set gas limit".to_string(),
        ))
    }

    pub fn with_libraries(&mut self, libraries: &[u8]) -> Result<&mut Self, TvmEmulatorError> {
        if libraries.is_empty() {
            return Ok(self);
        }
        if self.emulator.set_libraries(libraries)? {
            return Ok(self);
        }
        Err(TvmEmulatorError::EmulatorError(
            "Couldn't set libraries".to_string(),
        ))
    }
}

// use part
impl TvmEmulator {
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
        let stack_boc = build_stack_boc(stack)?;
        let run_result = self.emulator.run_get_method(method.to_id(), &stack_boc)?;
        let response = TvmEmulatorResponse::from_json(run_result.as_str())?;
        Ok(response)
    }
}
