//! Notification dispatcher — routes events to WS and persistence.

use std::sync::Arc;

use tracing;
use uuid::Uuid;

use filehub_core::config::NotificationsConfig;
use filehub_core::types::id::UserId;
use filehub_service::notification::service::NotificationService;

use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

use super::dedup::EventDeduplicator;
use super::persistence;
use super::priority::NotificationPriority;

/// Dispatches notifications to online users via WS and persists for offline users.
#[derive(Debug)]
pub struct NotificationDispatcher {
    /// Connection manager for sending WS messages
    connections: Arc<ConnectionManager>,
    /// Notification service for persistence
    notification_service: Arc<NotificationService>,
    /// Event deduplicator
    dedup: EventDeduplicator,
    /// Configuration
    config: NotificationsConfig,
}

impl NotificationDispatcher {
    /// Create a new dispatcher
    pub fn new(
        connections: Arc<ConnectionManager>,
        notification_service: Arc<NotificationService>,
        config: NotificationsConfig,
    ) -> Self {
        Self {
            connections,
            notification_service,
            dedup: EventDeduplicator::new(config.batch_window_ms),
            config,
        }
    }

    /// Dispatch a notification to a specific user.
    ///
    /// If the user is online, sends via WebSocket.
    /// If offline and `persist_for_offline` is enabled, saves to database.
    pub async fn dispatch_to_user(&self, user_id: UserId, msg: OutboundMessage) {
        if self.connections.is_online(user_id) {
            self.connections.send_to_user(user_id, msg).await;
        } else if self.config.persist_for_offline {
            if let Err(e) =
                persistence::persist_for_offline(&self.notification_service, user_id, &msg).await
            {
                tracing::error!(
                    "Failed to persist notification for offline user {}: {}",
                    user_id,
                    e
                );
            }
        }
    }

    /// Dispatch to multiple users
    pub async fn dispatch_to_users(&self, user_ids: &[Uuid], msg: OutboundMessage) {
        for uid in user_ids {
            self.dispatch_to_user(UserId::from(*uid), msg.clone()).await;
        }
    }

    /// Dispatch to a channel
    pub async fn dispatch_to_channel(&self, channel: &str, msg: OutboundMessage) {
        self.connections.send_to_channel(channel, msg).await;
    }

    /// Broadcast to all connected users
    pub async fn broadcast(&self, msg: OutboundMessage) {
        self.connections.broadcast(msg).await;
    }

    /// Dispatch with deduplication
    pub async fn dispatch_deduped(&self, user_id: UserId, dedup_key: &str, msg: OutboundMessage) {
        if self.dedup.should_dispatch(dedup_key) {
            self.dispatch_to_user(user_id, msg).await;
        } else {
            tracing::trace!("Notification deduplicated: key='{}'", dedup_key);
        }
    }

    /// Send an unread count update to a user
    pub async fn send_unread_count(&self, user_id: UserId, count: i64) {
        let msg = OutboundMessage::UnreadCount { count };
        self.connections.send_to_user(user_id, msg).await;
    }

    /// Cleanup dedup state
    pub fn cleanup_dedup(&self) {
        self.dedup.cleanup();
    }
}
