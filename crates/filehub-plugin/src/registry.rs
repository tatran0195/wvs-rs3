//! Plugin registry — stores loaded plugin instances and metadata.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::hooks::definitions::HookPoint;

/// Metadata about a loaded plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier.
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version string.
    pub version: String,
    /// Plugin description.
    pub description: String,
    /// Author or maintainer.
    pub author: String,
    /// List of hook points this plugin registers for.
    pub hooks: Vec<String>,
    /// Whether the plugin is currently enabled.
    pub enabled: bool,
    /// Load priority (lower = loaded first).
    pub priority: i32,
}

/// Trait that all plugins must implement.
#[async_trait::async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Returns plugin metadata.
    fn info(&self) -> PluginInfo;

    /// Called once when the plugin is loaded.
    async fn on_load(&self) -> Result<(), String>;

    /// Called when the plugin is enabled/started.
    async fn on_start(&self) -> Result<(), String>;

    /// Called when the plugin is disabled/stopped.
    async fn on_stop(&self) -> Result<(), String>;

    /// Called when the plugin is unloaded.
    async fn on_unload(&self) -> Result<(), String>;

    /// Returns the hook points this plugin wants to register for.
    fn registered_hooks(&self) -> Vec<HookPoint>;
}

/// Registry of all loaded plugins.
#[derive(Debug)]
pub struct PluginRegistry {
    /// Plugin ID → plugin instance.
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    /// Plugin ID → metadata.
    metadata: RwLock<HashMap<String, PluginInfo>>,
}

impl PluginRegistry {
    /// Creates a new empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a plugin.
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> Result<(), String> {
        let info = plugin.info();
        let id = info.id.clone();

        let mut plugins = self.plugins.write().await;
        let mut metadata = self.metadata.write().await;

        if plugins.contains_key(&id) {
            return Err(format!("Plugin '{}' is already registered", id));
        }

        info!(plugin_id = %id, name = %info.name, version = %info.version, "Registering plugin");

        plugins.insert(id.clone(), plugin);
        metadata.insert(id, info);

        Ok(())
    }

    /// Unregisters a plugin by ID.
    pub async fn unregister(&self, plugin_id: &str) -> Result<Arc<dyn Plugin>, String> {
        let mut plugins = self.plugins.write().await;
        let mut metadata = self.metadata.write().await;

        let plugin = plugins
            .remove(plugin_id)
            .ok_or_else(|| format!("Plugin '{}' not found", plugin_id))?;

        metadata.remove(plugin_id);

        info!(plugin_id = %plugin_id, "Plugin unregistered");

        Ok(plugin)
    }

    /// Gets a plugin by ID.
    pub async fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).cloned()
    }

    /// Lists all registered plugin metadata.
    pub async fn list(&self) -> Vec<PluginInfo> {
        let metadata = self.metadata.read().await;
        let mut infos: Vec<PluginInfo> = metadata.values().cloned().collect();
        infos.sort_by_key(|info| info.priority);
        infos
    }

    /// Returns plugin count.
    pub async fn count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Gets all registered plugin instances.
    pub async fn all_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// Checks whether a plugin is registered.
    pub async fn contains(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(plugin_id)
    }

    /// Enables a plugin by ID.
    pub async fn enable(&self, plugin_id: &str) -> Result<(), String> {
        let mut metadata = self.metadata.write().await;
        if let Some(info) = metadata.get_mut(plugin_id) {
            info.enabled = true;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    /// Disables a plugin by ID.
    pub async fn disable(&self, plugin_id: &str) -> Result<(), String> {
        let mut metadata = self.metadata.write().await;
        if let Some(info) = metadata.get_mut(plugin_id) {
            info.enabled = false;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    /// Checks whether a plugin is enabled.
    pub async fn is_enabled(&self, plugin_id: &str) -> bool {
        let metadata = self.metadata.read().await;
        metadata
            .get(plugin_id)
            .map(|info| info.enabled)
            .unwrap_or(false)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
