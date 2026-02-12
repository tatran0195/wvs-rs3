//! Session entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::presence::PresenceStatus;

/// An active user session.
///
/// Sessions are created on login and destroyed on logout, expiry,
/// or admin termination. Each session may hold a license seat.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    /// Unique session identifier.
    pub id: Uuid,
    /// The user this session belongs to.
    pub user_id: Uuid,
    /// SHA-256 hash of the access token.
    pub token_hash: String,
    /// SHA-256 hash of the refresh token (if issued).
    pub refresh_token_hash: Option<String>,
    /// IP address from which the session was created.
    pub ip_address: std::net::IpAddr,
    /// User-Agent header value.
    pub user_agent: Option<String>,
    /// Parsed device information (JSON).
    pub device_info: Option<serde_json::Value>,

    // -- License integration --
    /// FlexNet checkout ID (if a license seat is held).
    pub license_checkout_id: Option<String>,
    /// When the license seat was allocated.
    pub seat_allocated_at: Option<DateTime<Utc>>,
    /// Reference to the session that was kicked to make room for this one.
    pub overflow_kicked: Option<Uuid>,

    // -- Presence & WebSocket --
    /// Current presence status.
    pub presence_status: Option<PresenceStatus>,
    /// Whether a WebSocket connection is active.
    pub ws_connected: Option<bool>,
    /// When the WebSocket connection was established.
    pub ws_connected_at: Option<DateTime<Utc>>,

    // -- Termination --
    /// The admin who terminated this session (if applicable).
    pub terminated_by: Option<Uuid>,
    /// Reason for termination.
    pub terminated_reason: Option<String>,
    /// When the session was terminated.
    pub terminated_at: Option<DateTime<Utc>>,

    // -- Timestamps --
    /// When the session was created (login time).
    pub created_at: DateTime<Utc>,
    /// When the session expires (absolute timeout).
    pub expires_at: DateTime<Utc>,
    /// Last activity timestamp.
    pub last_activity: DateTime<Utc>,
}

impl Session {
    /// Check whether the session is still active (not terminated and not expired).
    pub fn is_active(&self) -> bool {
        self.terminated_at.is_none() && self.expires_at > Utc::now()
    }

    /// Check whether the session has been terminated by an admin.
    pub fn is_terminated(&self) -> bool {
        self.terminated_at.is_some()
    }

    /// Check whether the session has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// Check whether a WebSocket connection is currently active.
    pub fn is_ws_connected(&self) -> bool {
        self.ws_connected.unwrap_or(false)
    }

    /// Calculate how long the session has been idle (in seconds).
    pub fn idle_seconds(&self) -> i64 {
        (Utc::now() - self.last_activity).num_seconds().max(0)
    }
}

/// Data required to create a new session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSession {
    /// The user this session belongs to.
    pub user_id: Uuid,
    /// SHA-256 hash of the access token.
    pub token_hash: String,
    /// SHA-256 hash of the refresh token.
    pub refresh_token_hash: Option<String>,
    /// IP address of the client.
    pub ip_address: std::net::IpAddr,
    /// User-Agent header.
    pub user_agent: Option<String>,
    /// Parsed device info.
    pub device_info: Option<serde_json::Value>,
    /// When the session expires.
    pub expires_at: DateTime<Utc>,
}
