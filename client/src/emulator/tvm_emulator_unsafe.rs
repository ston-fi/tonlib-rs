use std::ffi::CString;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tonlib_sys::{
    emulator_set_verbosity_level, tvm_emulator_create, tvm_emulator_destroy,
    tvm_emulator_run_get_method, tvm_emulator_send_external_message,
    tvm_emulator_send_internal_message, tvm_emulator_set_c7, tvm_emulator_set_debug_enabled,
    tvm_emulator_set_gas_limit, tvm_emulator_set_libraries,
};

use crate::emulator::error::TvmEmulatorError;

#[derive(Debug)]
pub struct TvmEmulatorUnsafe {
    ptr: *mut std::os::raw::c_void,
}

unsafe impl Send for TvmEmulatorUnsafe {}

unsafe impl Sync for TvmEmulatorUnsafe {}

// construct part
impl TvmEmulatorUnsafe {
    pub fn new(
        code: &[u8],
        data: &[u8],
        vm_log_verbosity: u32,
    ) -> Result<TvmEmulatorUnsafe, TvmEmulatorError> {
        log::trace!("tvm_emulator_unsafe: creating...");
        let code = CString::new(STANDARD.encode(code))?;
        let data = CString::new(STANDARD.encode(data))?;

        let emulator: TvmEmulatorUnsafe = unsafe {
            let ptr = tvm_emulator_create(code.as_ptr(), data.as_ptr(), vm_log_verbosity);
            TvmEmulatorUnsafe { ptr }
        };
        if emulator.ptr.is_null() {
            log::trace!("tvm_emulator_unsafe: creating failed");
            Err(TvmEmulatorError::CreationFailed())
        } else {
            log::trace!("tvm_emulator_unsafe: created");
            Ok(emulator)
        }
    }

    pub fn set_global_verbosity_level(&mut self, level: u32) -> Result<bool, TvmEmulatorError> {
        let success: bool = unsafe { emulator_set_verbosity_level(level) };
        Ok(success)
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

// use part
impl TvmEmulatorUnsafe {
    pub fn run_get_method(
        &mut self,
        method_id: i32,
        stack_boc: &[u8],
    ) -> Result<String, TvmEmulatorError> {
        log::trace!("run_get_method_req: method_id: {method_id}, stack_boc: {stack_boc:?}");
        let data: CString = CString::new(STANDARD.encode(stack_boc))?;

        let json_str = unsafe {
            let c_str = tvm_emulator_run_get_method(self.ptr, method_id, data.as_ptr());
            convert_emulator_response(c_str)?
        };

        log::trace!(
            "run_get_method_rsp: method_id: {method_id}, stack_boc: {stack_boc:?}, rsp: {json_str}"
        );
        Ok(json_str.to_string())
    }

    pub fn send_internal_message(
        &mut self,
        message: &[u8],
        amount: u64,
    ) -> Result<String, TvmEmulatorError> {
        log::trace!("send_internal_message_req: msg: {message:?}, amount: {amount}");
        let message_encoded = CString::new(STANDARD.encode(message))?;

        let json_str = unsafe {
            let c_str =
                tvm_emulator_send_internal_message(self.ptr, message_encoded.into_raw(), amount);
            convert_emulator_response(c_str)?
        };

        log::trace!(
            "send_internal_message_rsp: msg: {message:?}, amount: {amount}, rsp: {json_str}"
        );
        Ok(json_str)
    }

    pub fn send_external_message(&mut self, message: &[u8]) -> Result<String, TvmEmulatorError> {
        log::trace!("send_internal_message_req: msg: {:?}", message);
        let message_encoded = CString::new(STANDARD.encode(message))?;

        let json_str = unsafe {
            let c_str = tvm_emulator_send_external_message(self.ptr, message_encoded.into_raw());
            convert_emulator_response(c_str)?
        };

        log::trace!("send_internal_message_rsp: msg: {message:?}, rsp: {json_str}");
        Ok(json_str)
    }
}

unsafe fn convert_emulator_response(
    c_str: *const std::os::raw::c_char,
) -> Result<String, TvmEmulatorError> {
    let json_str = std::ffi::CStr::from_ptr(c_str).to_str()?.to_string();
    libc::free(c_str as *mut std::ffi::c_void); // avoid memory leak after emulator strdup call
    Ok(json_str)
}

impl Drop for TvmEmulatorUnsafe {
    fn drop(&mut self) {
        unsafe { tvm_emulator_destroy(self.ptr) }
    }
}
