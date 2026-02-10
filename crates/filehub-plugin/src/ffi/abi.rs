//! FFI ABI definitions for dynamic plugins.
//!
//! Defines the C-compatible interface that dynamic plugins must export.

use std::os::raw::c_char;

/// FFI-safe plugin metadata.
#[repr(C)]
pub struct FfiPluginInfo {
    /// Plugin ID (null-terminated C string).
    pub id: *const c_char,
    /// Plugin name.
    pub name: *const c_char,
    /// Plugin version.
    pub version: *const c_char,
    /// Plugin description.
    pub description: *const c_char,
    /// Author.
    pub author: *const c_char,
    /// Priority.
    pub priority: i32,
}

/// FFI-safe hook result.
#[repr(C)]
pub enum FfiHookAction {
    /// Continue to next handler.
    Continue = 0,
    /// Halt execution.
    Halt = 1,
}

/// FFI-safe hook result with reason.
#[repr(C)]
pub struct FfiHookResult {
    /// Action.
    pub action: FfiHookAction,
    /// Halt reason (null-terminated, NULL if continue).
    pub reason: *const c_char,
    /// Output data as JSON string (null-terminated, NULL if none).
    pub output_json: *const c_char,
}

/// Type signature for the plugin factory function.
///
/// Dynamic plugins must export this function:
/// ```c
/// extern FfiPluginInfo* filehub_plugin_info();
/// ```
pub type FfiPluginInfoFn = unsafe extern "C" fn() -> *const FfiPluginInfo;

/// Type signature for plugin initialization.
pub type FfiPluginInitFn = unsafe extern "C" fn() -> i32;

/// Type signature for plugin cleanup.
pub type FfiPluginCleanupFn = unsafe extern "C" fn() -> i32;

/// Type signature for a hook handler.
///
/// `payload_json` is a null-terminated JSON string of the hook payload.
/// Returns an `FfiHookResult`.
pub type FfiHookHandlerFn = unsafe extern "C" fn(payload_json: *const c_char) -> FfiHookResult;
