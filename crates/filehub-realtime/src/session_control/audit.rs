//! Session action audit trail.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session control action for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAction {
    /// Action type
    pub action: SessionActionType,
    /// Target session ID
    pub session_id: Uuid,
    /// Target user ID
    pub target_user_id: Uuid,
    /// Who performed the action
    pub actor_id: Uuid,
    /// Reason for the action
    pub reason: String,
    /// IP address of the actor
    pub actor_ip: Option<String>,
    /// When the action occurred
    pub timestamp: DateTime<Utc>,
}

/// Types of session control actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionActionType {
    /// Single session terminated
    Terminate,
    /// Bulk termination
    BulkTerminate,
    /// All non-admin sessions terminated
    TerminateAll,
    /// Message sent to session
    SendMessage,
    /// Session limit updated
    LimitUpdated,
}

impl std::fmt::Display for SessionActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terminate => write!(f, "session_terminate"),
            Self::BulkTerminate => write!(f, "session_bulk_terminate"),
            Self::TerminateAll => write!(f, "session_terminate_all"),
            Self::SendMessage => write!(f, "session_send_message"),
            Self::LimitUpdated => write!(f, "session_limit_updated"),
        }
    }
}
