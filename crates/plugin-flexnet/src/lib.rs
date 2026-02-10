//! FlexNet license integration plugin for FileHub.
//!
//! Integrates with `license_proxy.dll` via FFI to provide seat-based
//! licensing. The auth flow is:
//!
//! - **Login**: create session → `LM_CheckOut(feature, session_id)`
//! - **Logout**: `LM_CheckIn(feature, session_id)` → destroy session
//!
//! When the `mock` feature is enabled (default), a mock implementation
//! is used instead of loading the real DLL, enabling development and
//! testing without a license server.

pub mod ffi;
pub mod hooks;
pub mod license;
pub mod plugin;

pub use plugin::FlexNetPlugin;
