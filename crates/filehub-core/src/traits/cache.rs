//! Cache provider trait for pluggable caching backends.

use std::time::Duration;

use async_trait::async_trait;

use crate::result::AppResult;

/// Trait for cache backends (Redis, in-memory, or layered).
///
/// All values are serialized as strings (JSON). The cache provider
/// is responsible for key prefixing and TTL enforcement.
#[async_trait]
pub trait CacheProvider: Send + Sync + std::fmt::Debug + 'static {
    /// Get a value by key. Returns `None` if the key does not exist or has expired.
    async fn get(&self, key: &str) -> AppResult<Option<String>>;

    /// Set a value with a TTL.
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> AppResult<()>;

    /// Set a value with the default TTL.
    async fn set_default(&self, key: &str, value: &str) -> AppResult<()>;

    /// Delete a key from the cache.
    async fn delete(&self, key: &str) -> AppResult<()>;

    /// Check whether a key exists in the cache.
    async fn exists(&self, key: &str) -> AppResult<bool>;

    /// Delete all keys matching a pattern (e.g., `"user:*"`).
    async fn delete_pattern(&self, pattern: &str) -> AppResult<u64>;

    /// Set a value only if the key does not already exist (NX).
    /// Returns `true` if the value was set, `false` if the key already existed.
    async fn set_nx(&self, key: &str, value: &str, ttl: Duration) -> AppResult<bool>;

    /// Increment an integer value by 1. Returns the new value.
    async fn incr(&self, key: &str) -> AppResult<i64>;

    /// Decrement an integer value by 1. Returns the new value.
    async fn decr(&self, key: &str) -> AppResult<i64>;

    /// Set the TTL on an existing key.
    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<bool>;

    /// Get a typed value by deserializing from JSON.
    async fn get_json<T: serde::de::DeserializeOwned + Send>(
        &self,
        key: &str,
    ) -> AppResult<Option<T>>
    where
        Self: Sized,
    {
        match self.get(key).await? {
            Some(value) => {
                let parsed = serde_json::from_str(&value)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Set a typed value by serializing to JSON.
    async fn set_json<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> AppResult<()>
    where
        Self: Sized,
    {
        let json = serde_json::to_string(value)?;
        self.set(key, &json, ttl).await
    }

    /// Check that the cache backend is reachable.
    async fn health_check(&self) -> AppResult<bool>;

    /// Flush all entries from the cache.
    async fn flush_all(&self) -> AppResult<()>;
}
