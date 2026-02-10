//! FFI module for FlexNet native library bindings.

pub mod bindings;
pub mod wrapper;

pub use bindings::FlexNetBindings;
pub use wrapper::FlexNetWrapper;
