//! Plugin system traits.

use async_trait::async_trait;
use serde_json::Value;

use crate::result::AppResult;

/// Result of a hook handler invocation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HookResult {
    /// Continue processing the hook chain.
    Continue,
    /// Continue but with modified data.
    ContinueWith(Value),
    /// Halt the hook chain and the parent operation.
    Halt(String),
}

/// Context passed to hook handlers during execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookContext {
    /// Name of the hook being invoked.
    pub hook_name: String,
    /// The payload data for this hook.
    pub data: Value,
    /// Identifier of the user who triggered the action (if any).
    pub user_id: Option<uuid::Uuid>,
    /// Additional metadata.
    pub metadata: Value,
}

impl HookContext {
    /// Create a new hook context.
    pub fn new(hook_name: impl Into<String>, data: Value) -> Self {
        Self {
            hook_name: hook_name.into(),
            data,
            user_id: None,
            metadata: Value::Object(serde_json::Map::new()),
        }
    }

    /// Set the user ID on this context.
    pub fn with_user(mut self, user_id: uuid::Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set additional metadata on this context.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Trait implemented by hook handlers (individual hook callbacks).
#[async_trait]
pub trait HookHandler: Send + Sync + 'static {
    /// The name of the hook this handler responds to.
    fn hook_name(&self) -> &str;

    /// Priority for ordering (lower executes first).
    fn priority(&self) -> i32 {
        100
    }

    /// Execute the hook handler.
    async fn execute(&self, context: &mut HookContext) -> AppResult<HookResult>;
}

/// Trait implemented by FileHub plugins.
///
/// A plugin provides metadata and registers its hook handlers.
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    /// Unique plugin identifier.
    fn id(&self) -> &str;

    /// Human-readable plugin name.
    fn name(&self) -> &str;

    /// Plugin version string.
    fn version(&self) -> &str;

    /// Plugin description.
    fn description(&self) -> &str {
        ""
    }

    /// Initialize the plugin. Called once at startup.
    async fn initialize(&mut self) -> AppResult<()>;

    /// Shut down the plugin. Called during graceful shutdown.
    async fn shutdown(&mut self) -> AppResult<()>;

    /// Return all hook handlers provided by this plugin.
    fn hook_handlers(&self) -> Vec<Box<dyn HookHandler>>;
}
