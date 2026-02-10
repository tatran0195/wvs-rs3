//! Session action audit trail for real-time events.

use tracing::info;
use uuid::Uuid;

/// Logs real-time session events for audit purposes.
#[derive(Debug, Clone)]
pub struct SessionActionAudit;

impl SessionActionAudit {
    /// Logs a connection event.
    pub fn log_connection(user_id: Uuid, session_id: Uuid, action: &str) {
        info!(
            user_id = %user_id,
            session_id = %session_id,
            action = %action,
            "RT session audit"
        );
    }

    /// Logs a subscription event.
    pub fn log_subscription(user_id: Uuid, channel: &str, action: &str) {
        info!(
            user_id = %user_id,
            channel = %channel,
            action = %action,
            "RT subscription audit"
        );
    }
}
