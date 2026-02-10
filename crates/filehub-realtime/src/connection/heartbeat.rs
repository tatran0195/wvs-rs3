//! Ping/pong heartbeat for WebSocket keepalive.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::time;
use tracing;

use super::handle::ConnectionHandle;

/// Heartbeat configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between pings
    pub ping_interval: Duration,
    /// Timeout before considering connection dead
    pub ping_timeout: Duration,
}

/// Run heartbeat loop for a connection.
///
/// Sends periodic pings and checks for pong responses.
/// Marks the connection as dead if pong is not received within timeout.
pub async fn run_heartbeat(handle: Arc<ConnectionHandle>, config: HeartbeatConfig) {
    let mut interval = time::interval(config.ping_interval);

    loop {
        interval.tick().await;

        if !handle.is_alive() {
            break;
        }

        // Check if last pong is within timeout
        let last_pong = *handle.last_pong.read().await;
        let elapsed = Utc::now() - last_pong;

        if let Ok(elapsed_std) = elapsed.to_std() {
            if elapsed_std > config.ping_timeout {
                tracing::warn!(
                    "Connection {} heartbeat timeout (last pong: {:?} ago)",
                    handle.id,
                    elapsed_std
                );
                handle.mark_dead();
                break;
            }
        }

        // Send ping
        let ping = crate::message::types::OutboundMessage::Ping {
            timestamp: Utc::now(),
        };

        if !handle.send(ping).await {
            tracing::debug!("Connection {} ping send failed, marking dead", handle.id);
            handle.mark_dead();
            break;
        }
    }

    tracing::debug!("Heartbeat loop ended for connection {}", handle.id);
}
