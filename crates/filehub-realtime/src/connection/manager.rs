//! Connection manager — handles connection lifecycle (add, remove, message routing).

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use filehub_core::config::RealtimeConfig;
use filehub_entity::user::UserRole;

use crate::channel::registry::ChannelRegistry;
use crate::message::types::{InboundMessage, OutboundMessage};
use crate::metrics::RealtimeMetrics;
use crate::presence::tracker::PresenceTracker;

use super::handle::{ConnectionHandle, ConnectionId};
use super::heartbeat::HeartbeatMonitor;
use super::pool::ConnectionPool;

/// Manages all active WebSocket connections.
#[derive(Debug)]
pub struct ConnectionManager {
    /// Connection pool.
    pool: Arc<ConnectionPool>,
    /// Channel registry.
    channels: Arc<ChannelRegistry>,
    /// Presence tracker.
    presence: Arc<PresenceTracker>,
    /// Metrics.
    metrics: Arc<RealtimeMetrics>,
    /// Configuration.
    config: RealtimeConfig,
}

impl ConnectionManager {
    /// Creates a new connection manager.
    pub fn new(
        config: RealtimeConfig,
        channels: Arc<ChannelRegistry>,
        presence: Arc<PresenceTracker>,
        metrics: Arc<RealtimeMetrics>,
    ) -> Self {
        Self {
            pool: Arc::new(ConnectionPool::new()),
            channels,
            presence,
            metrics,
            config,
        }
    }

    /// Registers a new authenticated connection.
    ///
    /// Returns the connection handle and a receiver for outbound messages.
    pub fn register(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        role: UserRole,
        username: String,
    ) -> (Arc<ConnectionHandle>, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel(self.config.channel_buffer_size);

        let handle = Arc::new(ConnectionHandle::new(
            user_id,
            session_id,
            role,
            username.clone(),
            tx,
        ));

        // Check max connections per user
        let existing = self.pool.get_user_connections(&user_id);
        if existing.len() >= self.config.max_connections_per_user {
            warn!(
                user_id = %user_id,
                count = existing.len(),
                max = self.config.max_connections_per_user,
                "User at max connections, oldest will be replaced"
            );
            // Close oldest connection
            if let Some(oldest) = existing.first() {
                oldest.mark_closed();
                self.pool.remove(&oldest.id);
            }
        }

        self.pool.add(handle.clone());
        self.presence.set_online(user_id, username);
        self.metrics.connection_opened();

        // Auto-subscribe to user's personal channel
        self.channels
            .subscribe(format!("user:{}", user_id), handle.id);

        info!(
            conn_id = %handle.id,
            user_id = %user_id,
            session_id = %session_id,
            "WebSocket connection registered"
        );

        (handle, rx)
    }

    /// Unregisters a connection and cleans up subscriptions.
    pub fn unregister(&self, conn_id: &ConnectionId) {
        if let Some(handle) = self.pool.remove(conn_id) {
            handle.mark_closed();

            // Unsubscribe from all channels
            self.channels.unsubscribe_all(*conn_id);

            // Update presence if no more connections
            let remaining = self.pool.get_user_connections(&handle.user_id);
            if remaining.is_empty() {
                self.presence.set_offline(handle.user_id);
            }

            self.metrics.connection_closed();

            info!(
                conn_id = %conn_id,
                user_id = %handle.user_id,
                "WebSocket connection unregistered"
            );
        }
    }

    /// Processes an inbound message from a client.
    pub async fn handle_inbound(&self, conn_id: &ConnectionId, raw_message: &str) {
        let handle = match self.pool.get(conn_id) {
            Some(h) => h,
            None => {
                warn!(conn_id = %conn_id, "Message from unknown connection");
                return;
            }
        };

        handle.touch();

        let msg: InboundMessage = match serde_json::from_str(raw_message) {
            Ok(m) => m,
            Err(e) => {
                let error_msg = OutboundMessage::Error {
                    code: "INVALID_MESSAGE".to_string(),
                    message: format!("Failed to parse message: {e}"),
                };
                let _ = handle
                    .send(serde_json::to_string(&error_msg).unwrap_or_default())
                    .await;
                return;
            }
        };

        match msg {
            InboundMessage::Subscribe { channel } => {
                self.handle_subscribe(&handle, &channel).await;
            }
            InboundMessage::Unsubscribe { channel } => {
                self.channels.unsubscribe(channel, handle.id);
                debug!(conn_id = %conn_id, "Unsubscribed from channel");
            }
            InboundMessage::Pong { .. } => {
                handle.touch();
            }
            InboundMessage::PresenceUpdate { status } => {
                self.presence.update_status(handle.user_id, status);
            }
            InboundMessage::MarkRead { notification_id } => {
                debug!(
                    conn_id = %conn_id,
                    notification_id = %notification_id,
                    "Mark read request"
                );
            }
            InboundMessage::Ack { message_id } => {
                debug!(conn_id = %conn_id, message_id = %message_id, "Message acknowledged");
            }
        }

        self.metrics.message_received();
    }

    /// Handles a subscribe request with permission checking.
    async fn handle_subscribe(&self, handle: &ConnectionHandle, channel: &str) {
        // Check subscription limits
        let current_subs = self.channels.subscription_count(handle.id);
        if current_subs >= self.config.max_subscriptions_per_connection {
            let error = OutboundMessage::Error {
                code: "MAX_SUBSCRIPTIONS".to_string(),
                message: format!(
                    "Maximum subscriptions ({}) reached",
                    self.config.max_subscriptions_per_connection
                ),
            };
            let _ = handle
                .send(serde_json::to_string(&error).unwrap_or_default())
                .await;
            return;
        }

        // Check channel permissions
        if !self.check_channel_permission(handle, channel) {
            let error = OutboundMessage::Error {
                code: "FORBIDDEN".to_string(),
                message: format!("Not authorized to subscribe to channel: {channel}"),
            };
            let _ = handle
                .send(serde_json::to_string(&error).unwrap_or_default())
                .await;
            return;
        }

        self.channels.subscribe(channel.to_string(), handle.id);

        let ack = OutboundMessage::Subscribed {
            channel: channel.to_string(),
        };
        let _ = handle
            .send(serde_json::to_string(&ack).unwrap_or_default())
            .await;

        debug!(
            conn_id = %handle.id,
            channel = %channel,
            "Subscribed to channel"
        );
    }

    /// Checks whether a connection has permission to subscribe to a channel.
    fn check_channel_permission(&self, handle: &ConnectionHandle, channel: &str) -> bool {
        // User's own channel — always allowed
        if channel == format!("user:{}", handle.user_id) {
            return true;
        }

        // Broadcast channel — always allowed
        if channel == "broadcast:all" {
            return true;
        }

        // Presence channel — always allowed
        if channel == "presence:global" {
            return true;
        }

        // Admin channels — admin only
        if channel.starts_with("admin:") {
            return matches!(handle.role, UserRole::Admin);
        }

        // Folder/file/upload/job channels — allowed for now; ACL checked at service layer
        if channel.starts_with("folder:")
            || channel.starts_with("file:")
            || channel.starts_with("upload:")
            || channel.starts_with("job:")
        {
            return true;
        }

        false
    }

    /// Sends a message to a specific user (all their connections).
    pub async fn send_to_user(&self, user_id: &Uuid, message: &OutboundMessage) {
        let connections = self.pool.get_user_connections(user_id);
        let msg = match serde_json::to_string(message) {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Failed to serialize outbound message");
                return;
            }
        };

        for conn in &connections {
            if let Err(e) = conn.send(msg.clone()).await {
                warn!(conn_id = %conn.id, error = %e, "Failed to send to user connection");
            }
        }

        self.metrics.message_sent_count(connections.len() as u64);
    }

    /// Sends a message to a specific session (all connections for that session).
    pub async fn send_to_session(&self, session_id: &Uuid, message: &OutboundMessage) {
        let connections = self.pool.get_session_connections(session_id);
        let msg = match serde_json::to_string(message) {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Failed to serialize outbound message");
                return;
            }
        };

        for conn in &connections {
            if let Err(e) = conn.send(msg.clone()).await {
                warn!(conn_id = %conn.id, error = %e, "Failed to send to session connection");
            }
        }
    }

    /// Broadcasts a message to a channel.
    pub async fn broadcast_to_channel(&self, channel: &str, message: &OutboundMessage) {
        let subscriber_ids = self.channels.get_subscribers(channel);
        let msg = match serde_json::to_string(message) {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Failed to serialize broadcast message");
                return;
            }
        };

        let mut sent = 0u64;
        for conn_id in &subscriber_ids {
            if let Some(handle) = self.pool.get(conn_id) {
                if let Err(e) = handle.send(msg.clone()).await {
                    warn!(conn_id = %conn_id, error = %e, "Failed to broadcast");
                } else {
                    sent += 1;
                }
            }
        }

        self.metrics.message_sent_count(sent);
    }

    /// Broadcasts a message to all connected clients.
    pub async fn broadcast_all(&self, message: &OutboundMessage) {
        let msg = match serde_json::to_string(message) {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Failed to serialize broadcast message");
                return;
            }
        };

        let all = self.pool.all_connections();
        for conn in &all {
            let _ = conn.send(msg.clone()).await;
        }

        self.metrics.message_sent_count(all.len() as u64);
    }

    /// Closes all connections for a session (used during session termination).
    pub async fn close_session_connections(&self, session_id: &Uuid) {
        let conns = self.pool.remove_session(session_id);
        for conn in &conns {
            conn.mark_closed();
            self.channels.unsubscribe_all(conn.id);
        }

        if !conns.is_empty() {
            info!(
                session_id = %session_id,
                count = conns.len(),
                "Closed session connections"
            );
        }
    }

    /// Closes all connections.
    pub async fn close_all(&self) {
        let all = self.pool.all_connections();
        for conn in &all {
            conn.mark_closed();
            self.pool.remove(&conn.id);
        }
        info!(count = all.len(), "All connections closed");
    }

    /// Returns the total connection count.
    pub fn connection_count(&self) -> usize {
        self.pool.connection_count()
    }

    /// Returns the number of unique connected users.
    pub fn user_count(&self) -> usize {
        self.pool.user_count()
    }

    /// Returns all connected user IDs.
    pub fn connected_user_ids(&self) -> Vec<Uuid> {
        self.pool.connected_user_ids()
    }

    /// Checks if a user is currently connected.
    pub fn is_user_connected(&self, user_id: &Uuid) -> bool {
        !self.pool.get_user_connections(user_id).is_empty()
    }

    /// Returns a reference to the heartbeat monitor config.
    pub fn heartbeat_config(&self) -> &RealtimeConfig {
        &self.config
    }

    /// Returns a reference to the connection pool.
    pub fn pool(&self) -> &Arc<ConnectionPool> {
        &self.pool
    }
}
