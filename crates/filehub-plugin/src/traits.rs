//! Simplified traits for plugin development.

use std::sync::Arc;

use async_trait::async_trait;

use crate::hooks::definitions::{HookPayload, HookPoint, HookResult};
use crate::hooks::registry::HookHandler;

/// Simplified hook handler trait that wraps the low-level `HookHandler`.
///
/// Plugin developers implement this instead of the raw trait for convenience.
#[async_trait]
pub trait SimpleHookHandler: Send + Sync + std::fmt::Debug {
    /// Returns the plugin ID.
    fn plugin_id(&self) -> &str;

    /// Returns the hook point this handler responds to.
    fn hook_point(&self) -> HookPoint;

    /// Returns the execution priority (lower = runs first).
    fn priority(&self) -> i32 {
        100
    }

    /// Handles the hook invocation.
    ///
    /// Return `HookResult::continue_execution` to proceed,
    /// or `HookResult::halt` to abort the operation (for `before_*` hooks).
    async fn handle(&self, payload: &HookPayload) -> HookResult;
}

/// Wrapper that adapts `SimpleHookHandler` to the `HookHandler` trait.
#[derive(Debug)]
pub struct SimpleHandlerAdapter {
    /// The inner handler.
    inner: Arc<dyn SimpleHookHandler>,
}

impl SimpleHandlerAdapter {
    /// Creates a new adapter wrapping a simple handler.
    pub fn new(handler: Arc<dyn SimpleHookHandler>) -> Self {
        Self { inner: handler }
    }

    /// Wraps a simple handler into an `Arc<dyn HookHandler>`.
    pub fn wrap(handler: Arc<dyn SimpleHookHandler>) -> Arc<dyn HookHandler> {
        Arc::new(Self::new(handler))
    }
}

#[async_trait]
impl HookHandler for SimpleHandlerAdapter {
    async fn handle(&self, payload: &HookPayload) -> HookResult {
        self.inner.handle(payload).await
    }

    fn plugin_id(&self) -> &str {
        self.inner.plugin_id()
    }

    fn priority(&self) -> i32 {
        self.inner.priority()
    }
}

/// A closure-based hook handler for quick handler creation.
pub struct ClosureHandler {
    /// Plugin ID.
    id: String,
    /// Priority.
    priority_val: i32,
    /// Handler function.
    handler: Arc<
        dyn Fn(
                &HookPayload,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = HookResult> + Send + '_>>
            + Send
            + Sync,
    >,
}

impl std::fmt::Debug for ClosureHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureHandler")
            .field("id", &self.id)
            .field("priority_val", &self.priority_val)
            .field("handler", &"<closure>")
            .finish()
    }
}

impl ClosureHandler {
    /// Creates a new closure-based handler.
    pub fn new<F, Fut>(plugin_id: &str, priority: i32, handler: F) -> Self
    where
        F: Fn(&HookPayload) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = HookResult> + Send + 'static,
    {
        let id = plugin_id.to_string();
        Self {
            id: id.clone(),
            priority_val: priority,
            handler: Arc::new(move |payload| {
                let fut = handler(payload);
                Box::pin(fut)
            }),
        }
    }
}

#[async_trait]
impl HookHandler for ClosureHandler {
    async fn handle(&self, payload: &HookPayload) -> HookResult {
        (self.handler)(payload).await
    }

    fn plugin_id(&self) -> &str {
        &self.id
    }

    fn priority(&self) -> i32 {
        self.priority_val
    }
}
