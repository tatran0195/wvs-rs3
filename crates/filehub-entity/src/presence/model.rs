//! Presence state value object.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PresenceStatus;

/// The complete presence state for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceState {
    /// User ID.
    pub user_id: Uuid,
    /// Display name.
    pub display_name: Option<String>,
    /// Current status.
    pub status: PresenceStatus,
    /// Last activity timestamp.
    pub last_activity: DateTime<Utc>,
    /// Whether a WebSocket is connected.
    pub ws_connected: bool,
    /// Number of active sessions.
    pub active_sessions: u32,
}
