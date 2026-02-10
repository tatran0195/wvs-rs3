//! WebSocket-level session termination.

use std::sync::Arc;

use chrono::Utc;
use tracing;

use filehub_core::types::id::SessionId;

use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

/// Terminate a session via WebSocket.
///
/// Sends a `session_terminated` message and then closes the connection.
pub async fn terminate_session_ws(
    connections: &Arc<ConnectionManager>,
    session_id: SessionId,
    reason: &str,
) {
    tracing::info!(
        "Terminating session {} via WebSocket: reason='{}'",
        session_id,
        reason
    );

    connections.close_session(session_id, reason).await;
}

/// Terminate multiple sessions
pub async fn terminate_sessions_ws(
    connections: &Arc<ConnectionManager>,
    session_ids: &[SessionId],
    reason: &str,
) {
    for sid in session_ids {
        terminate_session_ws(connections, *sid, reason).await;
    }
}
