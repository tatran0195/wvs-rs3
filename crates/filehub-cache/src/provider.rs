//! Cache manager that dispatches to the configured provider.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::info;

use filehub_core::config::cache::CacheConfig;
use filehub_core::error::AppError;
use filehub_core::result::AppResult;
use filehub_core::traits::cache::CacheProvider;

/// Cache manager that wraps the configured cache provider.
///
/// The provider is selected at construction time based on configuration.
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// The inner cache provider.
    inner: Arc<dyn CacheProvider>,
}

impl CacheManager {
    /// Create a new cache manager from configuration.
    pub async fn new(config: &CacheConfig) -> AppResult<Self> {
        let inner: Arc<dyn CacheProvider> = match config.provider.as_str() {
            #[cfg(feature = "redis-backend")]
            "redis" => {
                info!("Initializing Redis cache provider");
                let client = crate::redis::RedisClient::connect(&config.redis).await?;
                let provider =
                    crate::redis::RedisCacheProvider::new(client, config.default_ttl_seconds);
                Arc::new(provider)
            }
            #[cfg(feature = "memory")]
            "memory" => {
                info!("Initializing in-memory cache provider");
                let provider = crate::memory::MemoryCacheProvider::new(
                    &config.memory,
                    config.default_ttl_seconds,
                );
                Arc::new(provider)
            }
            other => {
                return Err(AppError::configuration(format!(
                    "Unknown cache provider: '{other}'. Supported: memory, redis"
                )));
            }
        };

        Ok(Self { inner })
    }

    /// Create a cache manager from an existing provider (for testing).
    pub fn from_provider(provider: Arc<dyn CacheProvider>) -> Self {
        Self { inner: provider }
    }

    /// Get a reference to the inner provider.
    pub fn provider(&self) -> &dyn CacheProvider {
        self.inner.as_ref()
    }
}

#[async_trait]
impl CacheProvider for CacheManager {
    async fn get(&self, key: &str) -> AppResult<Option<String>> {
        self.inner.get(key).await
    }

    async fn set(&self, key: &str, value: &str, ttl: Duration) -> AppResult<()> {
        self.inner.set(key, value, ttl).await
    }

    async fn set_default(&self, key: &str, value: &str) -> AppResult<()> {
        self.inner.set_default(key, value).await
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        self.inner.delete(key).await
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        self.inner.exists(key).await
    }

    async fn delete_pattern(&self, pattern: &str) -> AppResult<u64> {
        self.inner.delete_pattern(pattern).await
    }

    async fn set_nx(&self, key: &str, value: &str, ttl: Duration) -> AppResult<bool> {
        self.inner.set_nx(key, value, ttl).await
    }

    async fn incr(&self, key: &str) -> AppResult<i64> {
        self.inner.incr(key).await
    }

    async fn decr(&self, key: &str) -> AppResult<i64> {
        self.inner.decr(key).await
    }

    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<bool> {
        self.inner.expire(key, ttl).await
    }

    async fn health_check(&self) -> AppResult<bool> {
        self.inner.health_check().await
    }

    async fn flush_all(&self) -> AppResult<()> {
        self.inner.flush_all().await
    }
}
