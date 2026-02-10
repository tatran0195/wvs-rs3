//! Redis pub/sub bridge for multi-node deployments.
//!
//! Stub implementation — to be completed when Redis pub/sub is needed.

use serde::{Deserialize, Serialize};

/// Redis pub/sub bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisPubSubConfig {
    /// Redis URL
    pub url: String,
    /// Channel prefix
    pub prefix: String,
}

/// Redis pub/sub bridge (placeholder for multi-node support)
#[derive(Debug)]
pub struct RedisPubSubBridge {
    /// Configuration
    config: RedisPubSubConfig,
}

impl RedisPubSubBridge {
    /// Create a new Redis pub/sub bridge
    pub fn new(config: RedisPubSubConfig) -> Self {
        Self { config }
    }
}
