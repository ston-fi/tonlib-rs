use std::ffi::CString;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tonlib_sys::{
    tvm_emulator_create, tvm_emulator_destroy, tvm_emulator_run_get_method,
    tvm_emulator_send_external_message, tvm_emulator_send_internal_message, tvm_emulator_set_c7,
    tvm_emulator_set_debug_enabled, tvm_emulator_set_gas_limit, tvm_emulator_set_libraries,
};

use super::TvmEmulatorError;

#[derive(Debug)]
pub struct TvmEmulatorUnsafe {
    ptr: *mut ::std::os::raw::c_void,
}

unsafe impl Send for TvmEmulatorUnsafe {}

unsafe impl Sync for TvmEmulatorUnsafe {}

impl TvmEmulatorUnsafe {
    pub fn create(
        code: &[u8],
        data: &[u8],
        vm_log_verbosity: u32,
    ) -> Result<TvmEmulatorUnsafe, TvmEmulatorError> {
        let code = CString::new(STANDARD.encode(code))?;
        let data = CString::new(STANDARD.encode(data))?;

        let emulator: TvmEmulatorUnsafe = unsafe {
            let ptr = tvm_emulator_create(code.as_ptr(), data.as_ptr(), vm_log_verbosity);
            TvmEmulatorUnsafe { ptr }
        };
        if emulator.ptr.is_null() {
            Err(TvmEmulatorError::CreationFailed())
        } else {
            Ok(emulator)
        }
    }

    pub fn run_get_method(
        &mut self,
        method_id: i32,
        stack_boc: &[u8],
    ) -> Result<String, TvmEmulatorError> {
        let data: CString = CString::new(STANDARD.encode(stack_boc))?;
        let c_str = unsafe { tvm_emulator_run_get_method(self.ptr, method_id, data.as_ptr()) };

        let json_str: &str = unsafe { std::ffi::CStr::from_ptr(c_str).to_str()? };
        log::trace!("response {}", json_str);

        Ok(json_str.to_string())
    }

    pub fn send_internal_message(
        &mut self,
        message: &[u8],
        amount: u64,
    ) -> Result<String, TvmEmulatorError> {
        let message_encoded = CString::new(STANDARD.encode(message))?;
        let c_str = unsafe {
            tvm_emulator_send_internal_message(self.ptr, message_encoded.into_raw(), amount)
        };
        let json_str = unsafe { std::ffi::CStr::from_ptr(c_str).to_str() }?;
        log::trace!("response {}", json_str);
        Ok(json_str.to_string())
    }

    pub fn send_external_message(&mut self, message: &[u8]) -> Result<String, TvmEmulatorError> {
        let message_encoded = CString::new(STANDARD.encode(message))?;
        let c_str =
            unsafe { tvm_emulator_send_external_message(self.ptr, message_encoded.into_raw()) };
        let json_str = unsafe { std::ffi::CStr::from_ptr(c_str).to_str() }?;
        log::trace!("response {}", json_str);
        Ok(json_str.to_string())
    }

    pub fn set_libraries(&mut self, libs_boc: &[u8]) -> Result<bool, TvmEmulatorError> {
        let libs_encoded = CString::new(STANDARD.encode(libs_boc))?;
        let success = unsafe { tvm_emulator_set_libraries(self.ptr, libs_encoded.into_raw()) };
        Ok(success)
    }

    pub fn set_c7(
        &mut self,
        address: &[u8],
        unixtime: u32,
        balance: u64,
        rand_seed_hex: &[u8],
        config: &[u8],
    ) -> Result<bool, TvmEmulatorError> {
        let address_encoded = CString::new(address)?;
        let rand_seed_hex_encoded = CString::new(rand_seed_hex)?;
        let config_encoded = CString::new(STANDARD.encode(config))?;
        let success = unsafe {
            tvm_emulator_set_c7(
                self.ptr,
                address_encoded.into_raw(),
                unixtime,
                balance,
                rand_seed_hex_encoded.into_raw(),
                config_encoded.into_raw(),
            )
        };
        Ok(success)
    }

    pub fn set_gas_limit(&mut self, gas_limit: u64) -> bool {
        unsafe { tvm_emulator_set_gas_limit(self.ptr, gas_limit) }
    }

    pub fn set_debug_enabled(&mut self, enable: bool) -> bool {
        unsafe { tvm_emulator_set_debug_enabled(self.ptr, enable as i32) }
    }
}

impl Drop for TvmEmulatorUnsafe {
    fn drop(&mut self) {
        unsafe { tvm_emulator_destroy(self.ptr) }
    }
}
