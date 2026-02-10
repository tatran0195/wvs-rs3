//! Cache provider configuration.

use serde::{Deserialize, Serialize};

/// Top-level cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache provider type: `"memory"`, `"redis"`, or `"layered"`.
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Default TTL for cached entries in seconds.
    #[serde(default = "default_ttl")]
    pub default_ttl_seconds: u64,
    /// Redis-specific cache configuration.
    #[serde(default)]
    pub redis: RedisCacheConfig,
    /// In-memory cache configuration.
    #[serde(default)]
    pub memory: MemoryCacheConfig,
}

/// Redis cache backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisCacheConfig {
    /// Redis connection URL.
    #[serde(default = "default_redis_url")]
    pub url: String,
    /// Redis connection pool size.
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Key prefix for all FileHub cache keys.
    #[serde(default = "default_key_prefix")]
    pub key_prefix: String,
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            pool_size: default_pool_size(),
            key_prefix: default_key_prefix(),
        }
    }
}

/// In-memory cache backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    /// Maximum number of entries in the cache.
    #[serde(default = "default_max_capacity")]
    pub max_capacity: u64,
    /// TTL for in-memory entries in seconds.
    #[serde(default = "default_memory_ttl")]
    pub time_to_live_seconds: u64,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: default_max_capacity(),
            time_to_live_seconds: default_memory_ttl(),
        }
    }
}

fn default_provider() -> String {
    "memory".to_string()
}

fn default_ttl() -> u64 {
    300
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_key_prefix() -> String {
    "filehub:".to_string()
}

fn default_max_capacity() -> u64 {
    10000
}

fn default_memory_ttl() -> u64 {
    300
}
