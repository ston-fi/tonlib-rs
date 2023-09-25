use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde_json::Value;

use crate::tl::function::TonFunction;
use crate::tl::result::TonResult;

use super::error::TlError;

pub(crate) fn serialize_function(function: &TonFunction) -> Result<CString, TlError> {
    // TODO: Optimize to avoid copying
    let str = serde_json::to_string(function)?;
    let cstr = CString::new(str)?;
    Ok(cstr)
}

pub(crate) fn serialize_function_extra(
    function: &TonFunction,
    extra: &str,
) -> Result<CString, TlError> {
    let mut value = serde_json::to_value(function)?;
    let obj = value.as_object_mut().unwrap();
    obj.insert(String::from("@extra"), serde_json::Value::from(extra));
    // TODO: Optimize to avoid copying
    let str = serde_json::to_string(&value)?;
    let cstr = CString::new(str)?;
    Ok(cstr)
}

pub(crate) unsafe fn deserialize_result(c_str: *const c_char) -> Result<TonResult, TlError> {
    let cstr = CStr::from_ptr(c_str);
    // TODO: Optimize to avoid copying
    let str = cstr.to_str()?;
    let r = serde_json::from_str(str).unwrap(); // TODO: Transform error
    Ok(r)
}

pub(crate) unsafe fn deserialize_result_extra(
    c_str: *const c_char,
) -> (Result<TonResult, TlError>, Option<String>) {
    let cstr = CStr::from_ptr(c_str);
    // TODO: Optimize to avoid copying
    let str_result = cstr.to_str();
    if let Err(err) = str_result {
        return (Err(TlError::Utf8Error(err)), None);
    }
    let str = str_result.unwrap();
    let value_result: Result<Value, serde_json::Error> = serde_json::from_str(str);
    if let Err(err) = value_result {
        return (Err(TlError::SerdeJsonError(err)), None);
    }
    let value = value_result.unwrap();
    let extra: Option<String> = value
        .as_object()
        .and_then(|m| m.get("@extra"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let result: Result<TonResult, TlError> =
        serde_json::from_value(value).map_err(|e| TlError::SerdeJsonError(e));
    (result, extra)
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::tl::function::TonFunction;
    use crate::tl::result::TonResult;
    use crate::tl::serial::{deserialize_result_extra, serialize_function_extra};

    #[test]
    fn it_serializes_function_extra() {
        let func = TonFunction::SetLogVerbosityLevel {
            new_verbosity_level: 100500,
        };
        let cstr: CString = serialize_function_extra(&func, "some_extra").unwrap();
        assert_eq!(
            "{\"@extra\":\"some_extra\",\"@type\":\"setLogVerbosityLevel\",\"new_verbosity_level\":100500}",
            cstr.to_str().unwrap())
    }

    #[test]
    fn it_deserializes_result_extra() {
        let cstr = CString::new("{\"@extra\":\"some_extra\",\"@type\":\"logVerbosityLevel\",\"verbosity_level\":100500}").unwrap();
        let (result, extra) = unsafe { deserialize_result_extra(cstr.as_ptr()) };
        let expected = Some(String::from("some_extra"));
        assert_eq!(extra, expected);
        match result.unwrap() {
            TonResult::LogVerbosityLevel(verbosity_level) => {
                assert_eq!(verbosity_level.verbosity_level, 100500)
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    fn it_deserializes_options_info() {
        let cstr = CString::new(r#"{"@type":"options.info","config_info":
        {"@type":"options.configInfo","default_wallet_id":"698983191",
        "default_rwallet_init_public_key":"Puasxr0QfFZZnYISRphVse7XHKfW7pZU5SJarVHXvQ+rpzkD"},"@extra":"0"}"#).unwrap();
        let (_, extra) = unsafe { deserialize_result_extra(cstr.as_ptr()) };
        assert_eq!(extra, Some(String::from("0")));
    }
}
