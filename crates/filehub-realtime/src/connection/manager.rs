//! Connection manager — handles connection lifecycle.

use std::sync::Arc;

use serde_json;
use tokio::sync::mpsc;
use tracing;
use uuid::Uuid;

use filehub_core::types::id::{SessionId, UserId};
use filehub_entity::user::role::UserRole;

use crate::message::types::{InboundMessage, OutboundMessage};

use super::handle::{ConnectionHandle, ConnectionId, ConnectionInfo};
use super::pool::ConnectionPool;

/// Manages all WebSocket connections.
#[derive(Debug)]
pub struct ConnectionManager {
    /// Connection pool
    pool: ConnectionPool,
    /// Max connections per user
    max_per_user: usize,
    /// Max subscriptions per connection
    max_subscriptions: usize,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(max_per_user: usize, max_subscriptions: usize) -> Self {
        Self {
            pool: ConnectionPool::new(),
            max_per_user,
            max_subscriptions,
        }
    }

    /// Handle inbound message from connection
    pub async fn handle_inbound(&self, connection_id: &Uuid, text: &str) {
        let msg: InboundMessage = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(%connection_id, error = %e, "Failed to parse inbound message");
                return;
            }
        };

        match msg {
            InboundMessage::Subscribe { channel } => {
                if let Err(e) = self.subscribe(*connection_id, &channel).await {
                    tracing::warn!(%connection_id, channel, error = %e, "Subscription failed");
                }
            }
            InboundMessage::Unsubscribe { channel } => {
                self.unsubscribe(*connection_id, &channel).await;
            }
            InboundMessage::Heartbeat => {
                if let Some(handle) = self.pool.get(*connection_id) {
                    handle.touch().await;
                }
            }
            InboundMessage::Pong { .. } => {
                if let Some(handle) = self.pool.get(*connection_id) {
                    handle.record_pong().await;
                }
            }
            _ => {
                // Ignore other messages for now
                tracing::debug!(%connection_id, ?msg, "Unhandled inbound message");
            }
        }
    }

    /// Register a new connection.
    ///
    /// Returns `None` if the user already has max connections.
    pub fn register(
        &self,
        user_id: UserId,
        session_id: SessionId,
        user_role: UserRole,
        username: String,
        sender: mpsc::Sender<OutboundMessage>,
    ) -> Option<Arc<ConnectionHandle>> {
        let current = self.pool.user_connection_count(user_id);
        if current >= self.max_per_user {
            tracing::warn!(
                "User {} already has {} connections (max={}), rejecting",
                user_id,
                current,
                self.max_per_user
            );
            return None;
        }

        let handle = Arc::new(ConnectionHandle::new(
            user_id,
            session_id,
            user_role,
            username.clone(),
            sender,
        ));

        self.pool.add(Arc::clone(&handle));

        tracing::info!(
            "Connection registered: id={}, user='{}', session={}",
            handle.id,
            username,
            session_id
        );

        Some(handle)
    }

    /// Unregister a connection
    pub fn unregister(&self, connection_id: ConnectionId) {
        if let Some(handle) = self.pool.remove(connection_id) {
            handle.mark_dead();
            tracing::info!(
                "Connection unregistered: id={}, user='{}'",
                connection_id,
                handle.username
            );
        }
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(
        &self,
        connection_id: ConnectionId,
        msg: OutboundMessage,
    ) -> bool {
        if let Some(handle) = self.pool.get(connection_id) {
            handle.send(msg).await
        } else {
            false
        }
    }

    /// Send a message to all connections of a user
    pub async fn send_to_user(&self, user_id: UserId, msg: OutboundMessage) {
        let conns = self.pool.get_user_connections(user_id);
        for conn in conns {
            conn.send(msg.clone()).await;
        }
    }

    /// Send to all connections subscribed to a channel
    pub async fn send_to_channel(&self, channel: &str, msg: OutboundMessage) {
        let conns = self.pool.subscribed_to(channel).await;
        for conn in conns {
            conn.send(msg.clone()).await;
        }
    }

    /// Broadcast to ALL connections
    pub async fn broadcast(&self, msg: OutboundMessage) {
        for conn in self.pool.all_connections() {
            conn.send(msg.clone()).await;
        }
    }

    /// Subscribe a connection to a channel
    pub async fn subscribe(
        &self,
        connection_id: ConnectionId,
        channel: &str,
    ) -> Result<bool, &'static str> {
        let handle = self.pool.get(connection_id).ok_or("Connection not found")?;

        if handle.subscription_count().await >= self.max_subscriptions {
            return Err("Max subscriptions reached");
        }

        Ok(handle.subscribe(channel).await)
    }

    /// Unsubscribe a connection from a channel
    pub async fn unsubscribe(&self, connection_id: ConnectionId, channel: &str) -> bool {
        if let Some(handle) = self.pool.get(connection_id) {
            handle.unsubscribe(channel).await
        } else {
            false
        }
    }

    /// Close all connections for a session (admin termination)
    pub async fn close_session(&self, session_id: SessionId, reason: &str) {
        let msg = OutboundMessage::SessionTerminated {
            session_id,
            reason: reason.to_string(),
            terminated_at: chrono::Utc::now(),
        };

        let conns = self.pool.all_connections();
        for conn in conns {
            if conn.session_id == session_id {
                conn.send(msg.clone()).await;
                conn.mark_dead();
            }
        }

        self.pool.prune_dead();
    }

    /// Get all connections for a user
    pub fn get_user_connections(&self, user_id: UserId) -> Vec<Arc<ConnectionHandle>> {
        self.pool.get_user_connections(user_id)
    }

    /// Get connected user IDs
    pub fn connected_user_ids(&self) -> Vec<Uuid> {
        self.pool.connected_user_ids()
    }

    /// Check if a user is online (has at least one connection)
    pub fn is_online(&self, user_id: UserId) -> bool {
        self.pool.user_connection_count(user_id) > 0
    }

    /// Get total connection count
    pub fn total_connections(&self) -> usize {
        self.pool.total_count()
    }

    /// Get unique connected user count
    pub fn unique_users(&self) -> usize {
        self.pool.unique_user_count()
    }

    /// Prune dead connections
    pub fn prune_dead(&self) -> usize {
        self.pool.prune_dead()
    }

    /// Get info for all connections (admin view)
    pub async fn all_connection_info(&self) -> Vec<ConnectionInfo> {
        let mut infos = Vec::new();
        for conn in self.pool.all_connections() {
            infos.push(conn.info().await);
        }
        infos
    }
}
