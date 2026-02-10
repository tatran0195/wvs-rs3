//! Plugin context — services and resources available to plugin handlers.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Context passed to plugins providing access to FileHub services.
///
/// Plugins receive this when handling hooks, giving them access to
/// caching, database queries, and notification dispatch.
#[derive(Clone)]
pub struct PluginContext {
    /// Cache accessor.
    pub cache: Arc<dyn PluginCacheService>,
    /// Notification sender.
    pub notifications: Arc<dyn PluginNotificationService>,
    /// Database query service.
    pub database: Arc<dyn PluginDatabaseService>,
    /// Job queue service.
    pub jobs: Arc<dyn PluginJobService>,
}

impl std::fmt::Debug for PluginContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginContext").finish()
    }
}

/// Cache operations available to plugins.
#[async_trait::async_trait]
pub trait PluginCacheService: Send + Sync {
    /// Gets a value from cache.
    async fn get(&self, key: &str) -> Option<String>;
    /// Sets a value in cache.
    async fn set(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<(), String>;
    /// Deletes a value from cache.
    async fn delete(&self, key: &str) -> Result<(), String>;
}

/// Notification operations available to plugins.
#[async_trait::async_trait]
pub trait PluginNotificationService: Send + Sync {
    /// Sends a notification to a user.
    async fn send_to_user(
        &self,
        user_id: Uuid,
        title: &str,
        message: &str,
        category: &str,
    ) -> Result<(), String>;
    /// Sends a broadcast to all users.
    async fn broadcast(&self, title: &str, message: &str) -> Result<(), String>;
}

/// Database query operations available to plugins (read-only, safe subset).
#[async_trait::async_trait]
pub trait PluginDatabaseService: Send + Sync {
    /// Queries a single value by executing a safe predefined query.
    async fn query_value(
        &self,
        query_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

/// Job queue operations available to plugins.
#[async_trait::async_trait]
pub trait PluginJobService: Send + Sync {
    /// Enqueues a new background job.
    async fn enqueue(
        &self,
        job_type: &str,
        payload: serde_json::Value,
        queue: &str,
    ) -> Result<Uuid, String>;
}
