//! # filehub-plugin-sdk
//!
//! SDK for developing plugins for Suzuki FileHub.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use filehub_plugin_sdk::prelude::*;
//!
//! #[derive(Debug)]
//! struct MyPlugin;
//!
//! #[async_trait]
//! impl Plugin for MyPlugin {
//!     fn info(&self) -> PluginInfo {
//!         PluginInfo {
//!             id: "my-plugin".to_string(),
//!             name: "My Plugin".to_string(),
//!             version: "1.0.0".to_string(),
//!             description: "A sample plugin".to_string(),
//!             author: "Developer".to_string(),
//!             hooks: vec!["after_upload".to_string()],
//!             enabled: true,
//!             priority: 100,
//!         }
//!     }
//!
//!     async fn on_load(&self) -> Result<(), String> { Ok(()) }
//!     async fn on_start(&self) -> Result<(), String> { Ok(()) }
//!     async fn on_stop(&self) -> Result<(), String> { Ok(()) }
//!     async fn on_unload(&self) -> Result<(), String> { Ok(()) }
//!     fn registered_hooks(&self) -> Vec<HookPoint> { vec![HookPoint::AfterUpload] }
//! }
//! ```

pub mod exports;
pub mod macros;
pub mod traits;

/// Prelude for convenient imports.
pub mod prelude {
    pub use async_trait::async_trait;
    pub use filehub_plugin::api::context::{
        PluginCacheService, PluginContext, PluginDatabaseService, PluginJobService,
        PluginNotificationService,
    };
    pub use filehub_plugin::api::events::HookSubscription;
    pub use filehub_plugin::hooks::definitions::{HookAction, HookPayload, HookPoint, HookResult};
    pub use filehub_plugin::hooks::registry::HookHandler;
    pub use filehub_plugin::registry::{Plugin, PluginInfo};

    pub use crate::exports::PluginExport;
    pub use crate::traits::SimpleHookHandler;
}
