//! Hook system — registry, dispatcher, and typed hook definitions.

pub mod definitions;
pub mod dispatcher;
pub mod registry;

pub use definitions::{HookAction, HookPayload, HookPoint, HookResult};
pub use dispatcher::HookDispatcher;
pub use registry::HookRegistry;
