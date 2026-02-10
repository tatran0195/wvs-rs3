//! Individual WebSocket connection handle — send, receive, close.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use filehub_entity::user::UserRole;

/// Unique connection identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    /// Creates a new random connection ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a single authenticated WebSocket connection.
#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    /// Unique connection identifier.
    pub id: ConnectionId,
    /// Authenticated user ID.
    pub user_id: Uuid,
    /// Session ID from the JWT.
    pub session_id: Uuid,
    /// User role.
    pub role: UserRole,
    /// Username.
    pub username: String,
    /// Channel for sending messages to this connection.
    pub tx: mpsc::Sender<String>,
    /// When the connection was established.
    pub connected_at: DateTime<Utc>,
    /// Last activity timestamp.
    pub last_activity: Arc<std::sync::atomic::AtomicI64>,
    /// Whether the connection is still alive.
    pub alive: Arc<AtomicBool>,
}

impl ConnectionHandle {
    /// Creates a new connection handle.
    pub fn new(
        user_id: Uuid,
        session_id: Uuid,
        role: UserRole,
        username: String,
        tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            id: ConnectionId::new(),
            user_id,
            session_id,
            role,
            username,
            tx,
            connected_at: Utc::now(),
            last_activity: Arc::new(std::sync::atomic::AtomicI64::new(Utc::now().timestamp())),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Sends a text message to this connection.
    pub async fn send(&self, message: String) -> Result<(), String> {
        if !self.is_alive() {
            return Err("Connection is closed".to_string());
        }

        self.tx
            .send(message)
            .await
            .map_err(|e| format!("Send failed: {e}"))
    }

    /// Checks whether the connection is still alive.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Marks the connection as closed.
    pub fn mark_closed(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }

    /// Updates the last activity timestamp.
    pub fn touch(&self) {
        self.last_activity
            .store(Utc::now().timestamp(), Ordering::Relaxed);
    }

    /// Returns the last activity as a DateTime.
    pub fn last_activity_time(&self) -> DateTime<Utc> {
        let ts = self.last_activity.load(Ordering::Relaxed);
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    }

    /// Returns seconds since last activity.
    pub fn idle_seconds(&self) -> i64 {
        Utc::now().timestamp() - self.last_activity.load(Ordering::Relaxed)
    }
}
