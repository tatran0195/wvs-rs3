//! Hook dispatcher — fires hooks asynchronously and aggregates results.
//!
//! For `before_*` hooks:
//! - Handlers are called in priority order.
//! - If any handler returns `Halt`, execution stops and the main operation is aborted.
//! - If a handler returns `ContinueWith`, the modified data is merged into the payload.
//!
//! For `after_*` / `on_*` hooks:
//! - All handlers are called regardless of individual results.
//! - Handlers are called in priority order.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use filehub_core::error::AppError;

use super::definitions::{HookAction, HookPayload, HookPoint, HookResult};
use super::registry::HookRegistry;

/// Aggregated result of dispatching a hook to all handlers.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    /// Whether execution was halted.
    pub halted: bool,
    /// Halt reason (if halted).
    pub halt_reason: Option<String>,
    /// Plugin that halted (if halted).
    pub halted_by: Option<String>,
    /// All individual handler results.
    pub results: Vec<HookResult>,
    /// Merged modifications from `ContinueWith` results.
    pub modifications: std::collections::HashMap<String, serde_json::Value>,
}

/// Dispatches hooks to all registered handlers.
#[derive(Debug)]
pub struct HookDispatcher {
    /// Hook registry.
    registry: Arc<HookRegistry>,
}

impl HookDispatcher {
    /// Creates a new hook dispatcher.
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self { registry }
    }

    /// Dispatches a hook to all registered handlers.
    ///
    /// For `before_*` hooks, respects halt semantics.
    /// For other hooks, runs all handlers.
    pub async fn dispatch(&self, payload: &HookPayload) -> DispatchResult {
        let handlers = self.registry.get_handlers(&payload.hook).await;

        if handlers.is_empty() {
            return DispatchResult {
                halted: false,
                halt_reason: None,
                halted_by: None,
                results: Vec::new(),
                modifications: std::collections::HashMap::new(),
            };
        }

        debug!(
            hook = %payload.hook,
            handler_count = handlers.len(),
            "Dispatching hook"
        );

        let is_before_hook = payload.hook.is_before_hook();
        let mut results = Vec::new();
        let mut modifications = std::collections::HashMap::new();
        let mut halted = false;
        let mut halt_reason = None;
        let mut halted_by = None;

        for handler in &handlers {
            let result = match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                handler.handle(payload),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    error!(
                        hook = %payload.hook,
                        plugin_id = %handler.plugin_id(),
                        "Hook handler timed out after 30 seconds"
                    );
                    HookResult::continue_execution(handler.plugin_id())
                }
            };

            match &result.action {
                HookAction::Continue => {
                    debug!(
                        hook = %payload.hook,
                        plugin_id = %result.plugin_id,
                        "Handler returned Continue"
                    );
                }
                HookAction::ContinueWith(mods) => {
                    debug!(
                        hook = %payload.hook,
                        plugin_id = %result.plugin_id,
                        modifications = mods.len(),
                        "Handler returned ContinueWith"
                    );
                    modifications.extend(mods.clone());
                }
                HookAction::Halt { reason } => {
                    if is_before_hook {
                        info!(
                            hook = %payload.hook,
                            plugin_id = %result.plugin_id,
                            reason = %reason,
                            "Handler halted execution"
                        );
                        halted = true;
                        halt_reason = Some(reason.clone());
                        halted_by = Some(result.plugin_id.clone());
                        results.push(result);
                        break;
                    } else {
                        warn!(
                            hook = %payload.hook,
                            plugin_id = %result.plugin_id,
                            "Handler returned Halt for non-before hook, ignoring"
                        );
                    }
                }
            }

            results.push(result);
        }

        DispatchResult {
            halted,
            halt_reason,
            halted_by,
            results,
            modifications,
        }
    }

    /// Fires a hook and returns an error if halted.
    ///
    /// Convenience method for `before_*` hooks where halt should abort the operation.
    pub async fn fire_or_halt(&self, payload: &HookPayload) -> Result<DispatchResult, AppError> {
        let result = self.dispatch(payload).await;

        if result.halted {
            let reason = result
                .halt_reason
                .clone()
                .unwrap_or_else(|| "Hook halted execution".to_string());
            let plugin = result
                .halted_by
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            return Err(AppError::forbidden(format!(
                "Operation blocked by plugin '{}': {}",
                plugin, reason
            )));
        }

        Ok(result)
    }

    /// Fires a hook without checking for halt (for `after_*` / `on_*` hooks).
    ///
    /// All handlers run regardless of their results.
    pub async fn fire_and_forget(&self, payload: &HookPayload) {
        let _ = self.dispatch(payload).await;
    }

    /// Returns a reference to the hook registry.
    pub fn registry(&self) -> &Arc<HookRegistry> {
        &self.registry
    }
}
