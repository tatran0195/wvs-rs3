//! Redis pub/sub bridge for multi-node deployments.

#[cfg(feature = "redis-pubsub")]
pub mod implementation {
    use std::sync::Arc;

    use tracing::{error, info};

    use filehub_core::error::AppError;

    /// Redis pub/sub bridge for cross-node message relay.
    #[derive(Debug, Clone)]
    pub struct RedisPubSubBridge {
        /// Redis URL.
        url: String,
    }

    impl RedisPubSubBridge {
        /// Creates a new Redis pub/sub bridge.
        pub fn new(url: &str) -> Self {
            Self {
                url: url.to_string(),
            }
        }

        /// Publishes a message to a Redis channel.
        pub async fn publish(&self, channel: &str, message: &str) -> Result<(), AppError> {
            let client = redis::Client::open(self.url.as_str())
                .map_err(|e| AppError::internal(format!("Redis connection failed: {e}")))?;

            let mut conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| AppError::internal(format!("Redis connection failed: {e}")))?;

            redis::cmd("PUBLISH")
                .arg(channel)
                .arg(message)
                .query_async::<i64>(&mut conn)
                .await
                .map_err(|e| AppError::internal(format!("Redis PUBLISH failed: {e}")))?;

            Ok(())
        }
    }
}

#[cfg(not(feature = "redis-pubsub"))]
pub mod implementation {
    /// Stub Redis pub/sub bridge when redis feature is disabled.
    #[derive(Debug, Clone)]
    pub struct RedisPubSubBridge;

    impl RedisPubSubBridge {
        /// Creates a stub bridge.
        pub fn new(_url: &str) -> Self {
            Self
        }
    }
}

pub use implementation::RedisPubSubBridge;
