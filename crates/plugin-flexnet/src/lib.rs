//! FlexNet license integration plugin for FileHub.
//!
//! This plugin provides seat-based license management via FlexNet Publisher.
//! It supports both real FFI bindings to FlexNet DLLs and a mock implementation
//! for development and testing.

pub mod ffi;
pub mod hooks;
pub mod license;
pub mod plugin;

pub use plugin::FlexNetPlugin;
