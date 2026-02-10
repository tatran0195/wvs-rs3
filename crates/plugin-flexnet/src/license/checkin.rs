//! License checkin logic and cleanup.

use serde::{Deserialize, Serialize};

use filehub_core::types::id::SessionId;

/// Reason for a license checkin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckinReason {
    /// User logged out voluntarily
    UserLogout,
    /// Admin terminated the session
    AdminTermination,
    /// Session expired
    SessionExpired,
    /// Session idle timeout
    IdleTimeout,
    /// System shutdown
    SystemShutdown,
    /// Seat overflow kicked
    OverflowKick,
    /// Pool reconciliation
    Reconciliation,
}

impl std::fmt::Display for CheckinReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserLogout => write!(f, "user_logout"),
            Self::AdminTermination => write!(f, "admin_termination"),
            Self::SessionExpired => write!(f, "session_expired"),
            Self::IdleTimeout => write!(f, "idle_timeout"),
            Self::SystemShutdown => write!(f, "system_shutdown"),
            Self::OverflowKick => write!(f, "overflow_kick"),
            Self::Reconciliation => write!(f, "reconciliation"),
        }
    }
}

/// Parameters for a checkin request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinRequest {
    /// The session whose license should be checked in
    pub session_id: SessionId,
    /// The reason for the checkin
    pub reason: CheckinReason,
}
