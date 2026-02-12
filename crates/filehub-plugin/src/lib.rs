//! # filehub-plugin
//!
//! Plugin framework for FileHub. Provides:
//!
//! - Plugin lifecycle management (load, init, start, stop, unload)
//! - Hook registry with priority-ordered registration
//! - Hook dispatcher with Continue/Halt semantics
//! - Plugin API context exposing services to plugins
//! - Optional dynamic loading via `libloading` (later)

pub mod api;
pub mod hooks;
pub mod manager;
pub mod registry;

pub use api::context::PluginContext;
pub use hooks::definitions::{HookAction, HookPayload, HookPoint, HookResult};
pub use hooks::dispatcher::HookDispatcher;
pub use hooks::registry::HookRegistry;
pub use manager::PluginManager;
pub use registry::PluginRegistry;
