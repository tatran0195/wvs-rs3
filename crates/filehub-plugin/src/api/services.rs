//! Default implementations of plugin service traits, wrapping real FileHub services.

use std::sync::Arc;

use async_trait::async_trait;

use filehub_cache::provider::CacheManager;
use filehub_core::traits::CacheProvider;

use super::context::PluginCacheService;

/// Default cache service implementation for plugins wrapping `CacheManager`.
#[derive(Debug, Clone)]
pub struct DefaultPluginCacheService {
    /// Cache manager.
    cache: Arc<CacheManager>,
    /// Key prefix for plugin isolation.
    prefix: String,
}

impl DefaultPluginCacheService {
    /// Creates a new plugin cache service.
    pub fn new(cache: Arc<CacheManager>, plugin_id: &str) -> Self {
        Self {
            cache,
            prefix: format!("plugin:{}:", plugin_id),
        }
    }
}

#[async_trait]
impl PluginCacheService for DefaultPluginCacheService {
    async fn get(&self, key: &str) -> Option<String> {
        let full_key = format!("{}{}", self.prefix, key);
        let result: Result<Option<String>, _> = self.cache.get(&full_key).await;
        result.ok().flatten()
    }

    async fn set(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<(), String> {
        let full_key = format!("{}{}", self.prefix, key);
        self.cache
            .set(
                &full_key,
                value,
                std::time::Duration::from_secs(ttl_seconds),
            )
            .await
            .map_err(|e| format!("Cache set failed: {e}"))
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let full_key = format!("{}{}", self.prefix, key);
        self.cache
            .delete(&full_key)
            .await
            .map_err(|e| format!("Cache delete failed: {e}"))
    }
}
