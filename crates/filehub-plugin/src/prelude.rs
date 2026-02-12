//! Prelude for convenient imports.

pub use async_trait::async_trait;

pub use crate::api::context::{
    PluginCacheService, PluginContext, PluginDatabaseService, PluginJobService,
    PluginNotificationService,
};
pub use crate::api::events::HookSubscription;
pub use crate::hooks::definitions::{HookAction, HookPayload, HookPoint, HookResult};
pub use crate::hooks::registry::HookHandler;
pub use crate::registry::{Plugin, PluginInfo};

pub use crate::exports::PluginExport;
pub use crate::traits::SimpleHandlerAdapter;
pub use crate::traits::SimpleHookHandler;

pub use crate::plugin_info;
