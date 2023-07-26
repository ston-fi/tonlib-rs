use std::ffi::CStr;

use anyhow::Result;
use base64_serde::base64_serde_type;

use crate::tl::serial::{
    deserialize_result, deserialize_result_extra, serialize_function, serialize_function_extra,
};
use tonlib_sys::{
    tonlib_client_json_create, tonlib_client_json_destroy, tonlib_client_json_execute,
    tonlib_client_json_receive, tonlib_client_json_send,
};

mod function;
mod notification;
mod result;
mod serial;
pub mod stack;
pub mod types;

pub use function::TonFunction;
pub use notification::TonNotification;
pub use result::TonResult;

base64_serde_type!(Base64Standard, base64::STANDARD);

// Wrapper around ton client with support for TL data types
pub struct TlTonClient {
    ptr: *mut ::std::os::raw::c_void,
    tag: String,
}

impl TlTonClient {
    pub fn new(tag: &str) -> TlTonClient {
        let client: TlTonClient = unsafe {
            let ptr = tonlib_client_json_create();
            TlTonClient {
                ptr: ptr,
                tag: tag.into(),
            }
        };
        client
    }

    pub fn execute(&self, function: &TonFunction) -> Result<TonResult> {
        let f_str = serialize_function(function)?;
        log::trace!(
            "[{}] execute: {}",
            self.tag,
            f_str.to_str().unwrap_or("<Error decoding string as UTF-8>")
        );
        let result = unsafe {
            let c_str = tonlib_client_json_execute(self.ptr, f_str.as_ptr());
            log::trace!(
                "[{}] result: {}",
                self.tag,
                CStr::from_ptr(c_str)
                    .to_str()
                    .unwrap_or("<Error decoding string as UTF-8>")
            );
            deserialize_result(c_str)
        };
        result
    }

    pub fn send(&self, function: &TonFunction, extra: &str) -> Result<()> {
        let f_str = serialize_function_extra(function, extra)?;
        log::trace!(
            "[{}] send: {}",
            self.tag,
            f_str.to_str().unwrap_or("<Error decoding string as UTF-8>")
        );
        unsafe { tonlib_client_json_send(self.ptr, f_str.as_ptr()) };
        Ok(())
    }

    pub fn receive(&self, timeout: f64) -> Option<(Result<TonResult>, Option<String>)> {
        let c_str = unsafe { tonlib_client_json_receive(self.ptr, timeout) };
        if c_str.is_null() {
            None
        } else {
            let c_str_slice = unsafe { CStr::from_ptr(c_str) };
            if let Ok(c_str_str) = c_str_slice.to_str() {
                log::trace!("[{}] receive: {}", self.tag, c_str_str);
            } else {
                log::trace!("[{}] receive: <Error decoding string as UTF-8>", self.tag);
            }
            let c_str_bytes = c_str_slice.to_bytes();
            let (result, extra) =
                unsafe { deserialize_result_extra(c_str_bytes.as_ptr() as *const i8) };
            Some((result, extra))
        }
    }

    pub fn set_log_verbosity_level(verbosity_level: u32) {
        unsafe { tonlib_sys::tonlib_client_set_verbosity_level(verbosity_level) }
    }
}

impl Drop for TlTonClient {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                tonlib_client_json_destroy(self.ptr);
                self.ptr = std::ptr::null_mut();
            }
        }
    }
}

unsafe impl Send for TlTonClient {}

unsafe impl Sync for TlTonClient {}

#[cfg(test)]
mod tests {
    use crate::tl::function::TonFunction;
    use crate::tl::TlTonClient;

    #[test]
    fn set_log_verbosity_level_works() -> anyhow::Result<()> {
        let level = 1;
        TlTonClient::set_log_verbosity_level(level);
        Ok(())
    }

    #[test]
    fn it_executes_functions() -> anyhow::Result<()> {
        let client = TlTonClient::new("test");
        let get_logging = TonFunction::GetLogVerbosityLevel {};
        let _ = client.execute(&get_logging)?;
        Ok(())
    }
}
