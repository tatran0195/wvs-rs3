//! Hook registry — plugins register handlers by hook point with priority ordering.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::info;

use super::definitions::{HookPayload, HookPoint, HookResult};

/// Trait for hook handler implementations.
#[async_trait]
pub trait HookHandler: Send + Sync + std::fmt::Debug {
    /// Handles a hook invocation.
    async fn handle(&self, payload: &HookPayload) -> HookResult;

    /// Returns the plugin ID owning this handler.
    fn plugin_id(&self) -> &str;

    /// Returns the priority (lower = runs first).
    fn priority(&self) -> i32;
}

/// Entry in the hook registry.
#[derive(Debug)]
struct HookEntry {
    /// The handler.
    handler: Arc<dyn HookHandler>,
    /// Priority (lower = earlier execution).
    priority: i32,
    /// Plugin that registered this handler.
    plugin_id: String,
}

/// Registry of hook handlers organized by hook point.
#[derive(Debug)]
pub struct HookRegistry {
    /// Hook point → sorted list of handlers.
    handlers: RwLock<HashMap<HookPoint, Vec<HookEntry>>>,
}

impl HookRegistry {
    /// Creates a new empty hook registry.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a handler for a specific hook point.
    pub async fn register(&self, hook: HookPoint, handler: Arc<dyn HookHandler>) {
        let plugin_id = handler.plugin_id().to_string();
        let priority = handler.priority();

        let mut handlers = self.handlers.write().await;
        let entries = handlers.entry(hook.clone()).or_default();

        entries.push(HookEntry {
            handler,
            priority,
            plugin_id: plugin_id.clone(),
        });

        // Sort by priority (lower first)
        entries.sort_by_key(|e| e.priority);

        info!(
            hook = %hook,
            plugin_id = %plugin_id,
            priority = priority,
            "Hook handler registered"
        );
    }

    /// Unregisters all handlers for a specific plugin.
    pub async fn unregister_plugin(&self, plugin_id: &str) {
        let mut handlers = self.handlers.write().await;

        for entries in handlers.values_mut() {
            entries.retain(|e| e.plugin_id != plugin_id);
        }

        // Remove empty hook entries
        handlers.retain(|_, entries| !entries.is_empty());

        info!(plugin_id = %plugin_id, "All hooks unregistered for plugin");
    }

    /// Returns all handlers for a specific hook point, sorted by priority.
    pub async fn get_handlers(&self, hook: &HookPoint) -> Vec<Arc<dyn HookHandler>> {
        let handlers = self.handlers.read().await;
        handlers
            .get(hook)
            .map(|entries| entries.iter().map(|e| e.handler.clone()).collect())
            .unwrap_or_default()
    }

    /// Returns whether any handlers are registered for a hook point.
    pub async fn has_handlers(&self, hook: &HookPoint) -> bool {
        let handlers = self.handlers.read().await;
        handlers
            .get(hook)
            .map(|entries| !entries.is_empty())
            .unwrap_or(false)
    }

    /// Returns the number of handlers registered for a hook point.
    pub async fn handler_count(&self, hook: &HookPoint) -> usize {
        let handlers = self.handlers.read().await;
        handlers.get(hook).map(|entries| entries.len()).unwrap_or(0)
    }

    /// Returns all registered hook points.
    pub async fn registered_hooks(&self) -> Vec<HookPoint> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
