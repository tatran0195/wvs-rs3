//! WebSocket-level session termination — sends notice and closes connections.

use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

/// Handles WebSocket-level session termination.
#[derive(Debug)]
pub struct WsTerminator {
    /// Connection manager.
    connections: Arc<ConnectionManager>,
}

impl WsTerminator {
    /// Creates a new WS terminator.
    pub fn new(connections: Arc<ConnectionManager>) -> Self {
        Self { connections }
    }

    /// Terminates a session via WebSocket — sends notice, waits grace period, closes.
    pub async fn terminate_session(
        &self,
        session_id: Uuid,
        reason: &str,
        terminated_by: Option<Uuid>,
        grace_seconds: u32,
    ) {
        // Send termination notice
        let notice = OutboundMessage::SessionTerminated {
            session_id,
            reason: reason.to_string(),
            terminated_by,
            grace_seconds,
        };

        self.connections.send_to_session(&session_id, &notice).await;

        // Wait grace period
        if grace_seconds > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(grace_seconds as u64)).await;
        }

        // Force close connections
        self.connections
            .close_session_connections(&session_id)
            .await;

        info!(
            session_id = %session_id,
            "Session terminated via WebSocket"
        );
    }
}
