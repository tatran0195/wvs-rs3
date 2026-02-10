//! FFI module for license_proxy.dll bindings.

pub mod bindings;
pub mod mock;
pub mod wrapper;

pub use bindings::{LM_Result, LM_SUCCESS, LicenseManagerCtx, LicenseProxyApi};
pub use wrapper::LicenseManagerWrapper;
