//! Redis cache provider implementation.

use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;
use tracing::debug;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::traits::cache::CacheProvider;

use super::client::RedisClient;

/// Redis-backed cache provider.
#[derive(Debug, Clone)]
pub struct RedisCacheProvider {
    /// Redis client.
    client: RedisClient,
    /// Default TTL.
    default_ttl: Duration,
}

impl RedisCacheProvider {
    /// Create a new Redis cache provider.
    pub fn new(client: RedisClient, default_ttl_seconds: u64) -> Self {
        Self {
            client,
            default_ttl: Duration::from_secs(default_ttl_seconds),
        }
    }

    /// Map a Redis error to an AppError.
    fn map_err(e: redis::RedisError) -> AppError {
        AppError::with_source(ErrorKind::Cache, format!("Redis error: {e}"), e)
    }
}

#[async_trait]
impl CacheProvider for RedisCacheProvider {
    async fn get(&self, key: &str) -> AppResult<Option<String>> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let result: Option<String> = conn.get(&full_key).await.map_err(Self::map_err)?;
        Ok(result)
    }

    async fn set(&self, key: &str, value: &str, ttl: Duration) -> AppResult<()> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let _: () = conn
            .set_ex(&full_key, value, ttl.as_secs())
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn set_default(&self, key: &str, value: &str) -> AppResult<()> {
        self.set(key, value, self.default_ttl).await
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let _: () = conn.del(&full_key).await.map_err(Self::map_err)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let result: bool = conn.exists(&full_key).await.map_err(Self::map_err)?;
        Ok(result)
    }

    async fn delete_pattern(&self, pattern: &str) -> AppResult<u64> {
        let full_pattern = self.client.prefixed_key(pattern);
        let mut conn = self.client.conn_mut();

        // Use SCAN to find matching keys, then delete them.
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&full_pattern)
            .query_async(&mut conn)
            .await
            .map_err(Self::map_err)?;

        if keys.is_empty() {
            return Ok(0);
        }

        let count = keys.len() as u64;
        for key in &keys {
            let _: () = conn.del(key).await.map_err(Self::map_err)?;
        }

        debug!(pattern, count, "Deleted keys matching pattern");
        Ok(count)
    }

    async fn set_nx(&self, key: &str, value: &str, ttl: Duration) -> AppResult<bool> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();

        // SET key value EX ttl NX
        let result: Option<String> = redis::cmd("SET")
            .arg(&full_key)
            .arg(value)
            .arg("EX")
            .arg(ttl.as_secs())
            .arg("NX")
            .query_async(&mut conn)
            .await
            .map_err(Self::map_err)?;

        Ok(result.is_some())
    }

    async fn incr(&self, key: &str) -> AppResult<i64> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let result: i64 = conn.incr(&full_key, 1i64).await.map_err(Self::map_err)?;
        Ok(result)
    }

    async fn decr(&self, key: &str) -> AppResult<i64> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let result: i64 = conn.decr(&full_key, 1i64).await.map_err(Self::map_err)?;
        Ok(result)
    }

    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<bool> {
        let full_key = self.client.prefixed_key(key);
        let mut conn = self.client.conn_mut();
        let result: bool = conn
            .expire(&full_key, ttl.as_secs() as i64)
            .await
            .map_err(Self::map_err)?;
        Ok(result)
    }

    async fn health_check(&self) -> AppResult<bool> {
        let mut conn = self.client.conn_mut();
        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(Self::map_err)?;
        Ok(pong == "PONG")
    }

    async fn flush_all(&self) -> AppResult<()> {
        // Only flush keys with our prefix, not the entire Redis.
        self.delete_pattern("*").await?;
        Ok(())
    }
}
