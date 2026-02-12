//! Redis connection management.

use redis::Client;
use redis::aio::ConnectionManager;
use tracing::info;

use filehub_core::config::cache::RedisCacheConfig;
use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;

/// Redis client wrapper with connection management.
#[derive(Debug, Clone)]
pub struct RedisClient {
    /// Redis connection manager (pooled, reconnecting).
    conn: ConnectionManager,
    /// Key prefix for all keys.
    key_prefix: String,
}

impl RedisClient {
    /// Create a new Redis client from configuration.
    pub async fn connect(config: &RedisCacheConfig) -> AppResult<Self> {
        info!(url = %mask_redis_url(&config.url), "Connecting to Redis");

        let client = Client::open(config.url.as_str()).map_err(|e| {
            AppError::with_source(ErrorKind::Cache, "Failed to create Redis client", e)
        })?;

        let conn = ConnectionManager::new(client).await.map_err(|e| {
            AppError::with_source(ErrorKind::Cache, "Failed to connect to Redis", e)
        })?;

        info!("Successfully connected to Redis");
        Ok(Self {
            conn,
            key_prefix: config.key_prefix.clone(),
        })
    }

    /// Get a reference to the connection manager.
    pub fn connection(&self) -> &ConnectionManager {
        &self.conn
    }

    /// Get a mutable clone of the connection manager.
    pub fn conn_mut(&self) -> ConnectionManager {
        self.conn.clone()
    }

    /// Build a full key with the configured prefix.
    pub fn prefixed_key(&self, key: &str) -> String {
        format!("{}{key}", self.key_prefix)
    }

    /// Return the key prefix.
    pub fn prefix(&self) -> &str {
        &self.key_prefix
    }
}

/// Mask password in Redis URL for safe logging.
fn mask_redis_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
            if colon_pos > scheme_end {
                return format!("{}:****@{}", &url[..colon_pos], &url[at_pos + 1..]);
            }
        }
    }
    url.to_string()
}
