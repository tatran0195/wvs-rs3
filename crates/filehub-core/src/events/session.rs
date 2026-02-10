//! Session-related domain events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to user sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEvent {
    /// A user logged in and a session was created.
    Created {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
        /// The IP address of the login.
        ip_address: String,
    },
    /// A user logged out and the session was destroyed.
    Destroyed {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
        /// Why the session ended.
        reason: String,
    },
    /// A session was terminated by an admin.
    Terminated {
        /// The session ID.
        session_id: Uuid,
        /// The user whose session was terminated.
        user_id: Uuid,
        /// The admin who terminated it.
        terminated_by: Uuid,
        /// The reason for termination.
        reason: String,
    },
    /// A session expired due to timeout.
    Expired {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
    },
    /// A session's heartbeat was received (activity detected).
    HeartbeatReceived {
        /// The session ID.
        session_id: Uuid,
    },
    /// A session became idle.
    Idle {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
        /// How long the session has been idle in seconds.
        idle_seconds: u64,
    },
    /// A license seat was allocated for a session.
    SeatAllocated {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
    },
    /// A license seat was released from a session.
    SeatReleased {
        /// The session ID.
        session_id: Uuid,
        /// The user ID.
        user_id: Uuid,
    },
    /// A user's session limit was reached and overflow action taken.
    LimitReached {
        /// The user ID.
        user_id: Uuid,
        /// The limit that was reached.
        limit: u32,
        /// The overflow action taken.
        action: String,
    },
}
