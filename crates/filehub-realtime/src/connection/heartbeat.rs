//! Ping/pong heartbeat for WebSocket keepalive and idle detection.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::interval;
use tracing::{debug, warn};

use filehub_core::config::RealtimeConfig;

use super::handle::ConnectionHandle;

/// Manages heartbeat ping/pong for a single connection.
pub struct HeartbeatMonitor {
    /// Ping interval.
    ping_interval: Duration,
    /// Timeout before considering a connection dead.
    ping_timeout: Duration,
}

impl HeartbeatMonitor {
    /// Creates a new heartbeat monitor from configuration.
    pub fn new(config: &RealtimeConfig) -> Self {
        Self {
            ping_interval: Duration::from_secs(config.ping_interval_seconds as u64),
            ping_timeout: Duration::from_secs(config.ping_timeout_seconds as u64),
        }
    }

    /// Runs the heartbeat loop for a connection.
    ///
    /// Returns when the connection is considered dead or closed.
    pub async fn run(&self, handle: Arc<ConnectionHandle>) {
        let mut ticker = interval(self.ping_interval);

        loop {
            ticker.tick().await;

            if !handle.is_alive() {
                debug!(conn_id = %handle.id, "Connection closed, stopping heartbeat");
                return;
            }

            // Check if connection has been idle too long
            let idle = handle.idle_seconds();
            if idle > self.ping_timeout.as_secs() as i64 {
                warn!(
                    conn_id = %handle.id,
                    user_id = %handle.user_id,
                    idle_seconds = idle,
                    "Connection heartbeat timeout, marking as dead"
                );
                handle.mark_closed();
                return;
            }

            // Send ping
            let ping_msg = serde_json::json!({
                "type": "ping",
                "timestamp": chrono::Utc::now().timestamp()
            })
            .to_string();

            if let Err(e) = handle.send(ping_msg).await {
                warn!(
                    conn_id = %handle.id,
                    error = %e,
                    "Failed to send ping, marking connection as dead"
                );
                handle.mark_closed();
                return;
            }

            debug!(conn_id = %handle.id, "Ping sent");
        }
    }
}
