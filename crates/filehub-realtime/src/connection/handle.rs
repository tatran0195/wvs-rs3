//! Individual WebSocket connection handle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use filehub_core::types::id::{SessionId, UserId};
use filehub_entity::user::role::UserRole;

use crate::message::types::OutboundMessage;

/// Unique connection identifier
pub type ConnectionId = Uuid;

/// A handle to a single WebSocket connection.
///
/// Holds the sender channel for pushing messages to the client,
/// plus metadata about the connected user and session.
#[derive(Debug)]
pub struct ConnectionHandle {
    /// Unique connection ID
    pub id: ConnectionId,
    /// User who owns this connection
    pub user_id: UserId,
    /// Session this connection belongs to
    pub session_id: SessionId,
    /// User's role (cached for quick checks)
    pub user_role: UserRole,
    /// Username (cached for display)
    pub username: String,
    /// Sender for outbound messages
    pub sender: mpsc::Sender<OutboundMessage>,
    /// Channels this connection is subscribed to
    pub subscriptions: tokio::sync::RwLock<Vec<String>>,
    /// When the connection was established
    pub connected_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: tokio::sync::RwLock<DateTime<Utc>>,
    /// Last pong received
    pub last_pong: tokio::sync::RwLock<DateTime<Utc>>,
    /// Whether the connection is still alive
    pub alive: AtomicBool,
}

impl ConnectionHandle {
    /// Create a new connection handle
    pub fn new(
        user_id: UserId,
        session_id: SessionId,
        user_role: UserRole,
        username: String,
        sender: mpsc::Sender<OutboundMessage>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            session_id,
            user_role,
            username,
            sender,
            subscriptions: tokio::sync::RwLock::new(Vec::new()),
            connected_at: now,
            last_activity: tokio::sync::RwLock::new(now),
            last_pong: tokio::sync::RwLock::new(now),
            alive: AtomicBool::new(true),
        }
    }

    /// Send an outbound message to this connection
    pub async fn send(&self, msg: OutboundMessage) -> bool {
        if !self.is_alive() {
            return false;
        }
        match self.sender.try_send(msg) {
            Ok(_) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!("Connection {} send buffer full, dropping message", self.id);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.mark_dead();
                false
            }
        }
    }

    /// Check if connection is alive
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Mark connection as dead
    pub fn mark_dead(&self) {
        self.alive.store(false, Ordering::SeqCst);
    }

    /// Update last activity timestamp
    pub async fn touch(&self) {
        let mut la = self.last_activity.write().await;
        *la = Utc::now();
    }

    /// Record a pong response
    pub async fn record_pong(&self) {
        let mut lp = self.last_pong.write().await;
        *lp = Utc::now();
    }

    /// Add a subscription
    pub async fn subscribe(&self, channel: &str) -> bool {
        let mut subs = self.subscriptions.write().await;
        if subs.contains(&channel.to_string()) {
            return false;
        }
        subs.push(channel.to_string());
        true
    }

    /// Remove a subscription
    pub async fn unsubscribe(&self, channel: &str) -> bool {
        let mut subs = self.subscriptions.write().await;
        let before = subs.len();
        subs.retain(|s| s != channel);
        subs.len() < before
    }

    /// Get current subscription count
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Check if subscribed to a channel
    pub async fn is_subscribed(&self, channel: &str) -> bool {
        self.subscriptions.read().await.iter().any(|s| s == channel)
    }

    /// Get a snapshot of connection info
    pub async fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: self.id,
            user_id: self.user_id,
            session_id: self.session_id,
            username: self.username.clone(),
            role: self.user_role.clone(),
            connected_at: self.connected_at,
            last_activity: *self.last_activity.read().await,
            subscriptions: self.subscriptions.read().await.clone(),
            alive: self.is_alive(),
        }
    }
}

/// Snapshot of connection info (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Connection ID
    pub id: ConnectionId,
    /// User ID
    pub user_id: UserId,
    /// Session ID
    pub session_id: SessionId,
    /// Username
    pub username: String,
    /// Role
    pub role: UserRole,
    /// Connected at
    pub connected_at: DateTime<Utc>,
    /// Last activity
    pub last_activity: DateTime<Utc>,
    /// Subscriptions
    pub subscriptions: Vec<String>,
    /// Is alive
    pub alive: bool,
}
