//! FFI safety wrappers — converts between FFI types and Rust types.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::hooks::definitions::{HookAction, HookResult};

use super::abi::{FfiHookAction, FfiHookResult};

/// Safely converts a C string pointer to a Rust `String`.
///
/// Returns `None` if the pointer is null.
pub fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

/// Converts a Rust string to a `CString`.
///
/// Returns `None` if the string contains null bytes.
pub fn string_to_c_string(s: &str) -> Option<CString> {
    CString::new(s).ok()
}

/// Converts an FFI hook result to a Rust `HookResult`.
pub fn ffi_result_to_hook_result(ffi_result: &FfiHookResult, plugin_id: &str) -> HookResult {
    match ffi_result.action {
        FfiHookAction::Continue => {
            let output = c_str_to_string(ffi_result.output_json)
                .and_then(|json| serde_json::from_str(&json).ok());

            HookResult {
                action: HookAction::Continue,
                output,
                plugin_id: plugin_id.to_string(),
            }
        }
        FfiHookAction::Halt => {
            let reason = c_str_to_string(ffi_result.reason)
                .unwrap_or_else(|| "No reason provided".to_string());

            HookResult {
                action: HookAction::Halt { reason },
                output: None,
                plugin_id: plugin_id.to_string(),
            }
        }
    }
}
