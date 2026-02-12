//! Export helpers for building plugin registration bundles.

use std::sync::Arc;

use crate::hooks::definitions::HookPoint;
use crate::hooks::registry::HookHandler;
use crate::registry::Plugin;

/// A bundle describing a fully assembled plugin ready for registration.
#[derive(Debug)]
pub struct PluginExport {
    /// The plugin instance.
    pub plugin: Arc<dyn Plugin>,
    /// Hook handlers to register, keyed by hook point.
    pub handlers: Vec<(HookPoint, Arc<dyn HookHandler>)>,
}

impl PluginExport {
    /// Creates a new plugin export with no handlers.
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            handlers: Vec::new(),
        }
    }

    /// Adds a hook handler.
    pub fn with_handler(mut self, hook: HookPoint, handler: Arc<dyn HookHandler>) -> Self {
        self.handlers.push((hook, handler));
        self
    }

    /// Adds multiple handlers.
    pub fn with_handlers(mut self, handlers: Vec<(HookPoint, Arc<dyn HookHandler>)>) -> Self {
        self.handlers.extend(handlers);
        self
    }
}

/// Builder for constructing plugin exports incrementally.
#[derive(Debug)]
pub struct PluginExportBuilder {
    /// The plugin.
    plugin: Arc<dyn Plugin>,
    /// Accumulated handlers.
    handlers: Vec<(HookPoint, Arc<dyn HookHandler>)>,
}

impl PluginExportBuilder {
    /// Creates a new builder with the given plugin.
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            handlers: Vec::new(),
        }
    }

    /// Registers a handler for a hook point.
    pub fn on(mut self, hook: HookPoint, handler: Arc<dyn HookHandler>) -> Self {
        self.handlers.push((hook, handler));
        self
    }

    /// Builds the final export.
    pub fn build(self) -> PluginExport {
        PluginExport {
            plugin: self.plugin,
            handlers: self.handlers,
        }
    }
}
