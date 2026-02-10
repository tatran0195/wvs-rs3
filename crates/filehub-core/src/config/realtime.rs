//! Real-time WebSocket engine configuration.

use serde::{Deserialize, Serialize};

/// Real-time (WebSocket) engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    /// Maximum WebSocket connections per user.
    #[serde(default = "default_max_connections_per_user")]
    pub max_connections_per_user: usize,
    /// Internal channel buffer size for broadcast channels.
    #[serde(default = "default_channel_buffer")]
    pub channel_buffer_size: usize,
    /// WebSocket ping interval in seconds.
    #[serde(default = "default_ping_interval")]
    pub ping_interval_seconds: u64,
    /// WebSocket ping timeout in seconds.
    #[serde(default = "default_ping_timeout")]
    pub ping_timeout_seconds: u64,
    /// Maximum channel subscriptions per connection.
    #[serde(default = "default_max_subscriptions")]
    pub max_subscriptions_per_connection: usize,
    /// Notification-specific settings.
    #[serde(default)]
    pub notifications: NotificationRealtimeConfig,
}

/// Notification delivery settings for the real-time engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRealtimeConfig {
    /// Whether to persist notifications for offline users.
    #[serde(default = "default_true")]
    pub persist_for_offline: bool,
    /// Maximum stored notifications per user.
    #[serde(default = "default_max_stored")]
    pub max_stored_per_user: u64,
    /// Number of days after which stored notifications are cleaned up.
    #[serde(default = "default_cleanup_days")]
    pub cleanup_after_days: u32,
    /// Deduplication batch window in milliseconds.
    #[serde(default = "default_batch_window")]
    pub batch_window_ms: u64,
}

impl Default for NotificationRealtimeConfig {
    fn default() -> Self {
        Self {
            persist_for_offline: true,
            max_stored_per_user: default_max_stored(),
            cleanup_after_days: default_cleanup_days(),
            batch_window_ms: default_batch_window(),
        }
    }
}

fn default_max_connections_per_user() -> usize {
    5
}

fn default_channel_buffer() -> usize {
    256
}

fn default_ping_interval() -> u64 {
    30
}

fn default_ping_timeout() -> u64 {
    10
}

fn default_max_subscriptions() -> usize {
    50
}

fn default_true() -> bool {
    true
}

fn default_max_stored() -> u64 {
    1000
}

fn default_cleanup_days() -> u32 {
    30
}

fn default_batch_window() -> u64 {
    500
}
