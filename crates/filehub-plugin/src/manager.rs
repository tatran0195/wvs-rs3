//! Plugin manager — lifecycle management for all plugins.

use std::sync::Arc;

use tracing::{error, info, warn};

use filehub_core::error::AppError;

use crate::hooks::definitions::HookPoint;
use crate::hooks::dispatcher::HookDispatcher;
use crate::hooks::registry::{HookHandler, HookRegistry};
use crate::registry::{Plugin, PluginRegistry};

/// Manages the full lifecycle of plugins: load, init, start, stop, unload.
#[derive(Debug)]
pub struct PluginManager {
    /// Plugin registry.
    plugin_registry: Arc<PluginRegistry>,
    /// Hook registry.
    hook_registry: Arc<HookRegistry>,
    /// Hook dispatcher.
    hook_dispatcher: Arc<HookDispatcher>,
}

impl PluginManager {
    /// Creates a new plugin manager.
    pub fn new() -> Self {
        let hook_registry = Arc::new(HookRegistry::new());
        let hook_dispatcher = Arc::new(HookDispatcher::new(hook_registry.clone()));

        Self {
            plugin_registry: Arc::new(PluginRegistry::new()),
            hook_registry,
            hook_dispatcher,
        }
    }

    /// Loads and starts a compiled-in plugin.
    pub async fn load_plugin(
        &self,
        plugin: Arc<dyn Plugin>,
        handlers: Vec<(HookPoint, Arc<dyn HookHandler>)>,
    ) -> Result<(), AppError> {
        let info = plugin.info();
        let plugin_id = info.id.clone();

        // Load
        plugin.on_load().await.map_err(|e| {
            AppError::internal(format!("Plugin '{}' load failed: {}", plugin_id, e))
        })?;

        // Register
        self.plugin_registry
            .register(plugin.clone())
            .await
            .map_err(|e| AppError::internal(format!("Plugin registration failed: {e}")))?;

        // Register hooks
        for (hook_point, handler) in handlers {
            self.hook_registry.register(hook_point, handler).await;
        }

        // Start
        plugin.on_start().await.map_err(|e| {
            error!(plugin_id = %plugin_id, error = %e, "Plugin start failed");
            AppError::internal(format!("Plugin '{}' start failed: {}", plugin_id, e))
        })?;

        info!(
            plugin_id = %plugin_id,
            name = %info.name,
            version = %info.version,
            hooks = info.hooks.len(),
            "Plugin loaded and started"
        );

        Ok(())
    }

    /// Stops and unloads a plugin.
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), AppError> {
        let plugin = self
            .plugin_registry
            .get(plugin_id)
            .await
            .ok_or_else(|| AppError::not_found(format!("Plugin '{}' not found", plugin_id)))?;

        // Stop
        if let Err(e) = plugin.on_stop().await {
            warn!(
                plugin_id = %plugin_id,
                error = %e,
                "Plugin stop returned error"
            );
        }

        // Unregister hooks
        self.hook_registry.unregister_plugin(plugin_id).await;

        // Unregister plugin
        self.plugin_registry
            .unregister(plugin_id)
            .await
            .map_err(|e| AppError::internal(format!("Plugin unregistration failed: {e}")))?;

        // Unload
        if let Err(e) = plugin.on_unload().await {
            warn!(
                plugin_id = %plugin_id,
                error = %e,
                "Plugin unload returned error"
            );
        }

        info!(plugin_id = %plugin_id, "Plugin unloaded");

        Ok(())
    }

    /// Stops and unloads all plugins.
    pub async fn unload_all(&self) -> Result<(), AppError> {
        let plugins = self.plugin_registry.list().await;

        for info in &plugins {
            if let Err(e) = self.unload_plugin(&info.id).await {
                error!(
                    plugin_id = %info.id,
                    error = %e,
                    "Error unloading plugin"
                );
            }
        }

        info!("All plugins unloaded");
        Ok(())
    }

    /// Returns the hook dispatcher for firing hooks.
    pub fn dispatcher(&self) -> &Arc<HookDispatcher> {
        &self.hook_dispatcher
    }

    /// Returns the hook registry.
    pub fn hook_registry(&self) -> &Arc<HookRegistry> {
        &self.hook_registry
    }

    /// Returns the plugin registry.
    pub fn plugin_registry(&self) -> &Arc<PluginRegistry> {
        &self.plugin_registry
    }

    /// Lists all loaded plugins.
    pub async fn list_plugins(&self) -> Vec<crate::registry::PluginInfo> {
        self.plugin_registry.list().await
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
