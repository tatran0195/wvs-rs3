//! Central notification dispatcher — routes notifications to online/offline users.

use std::sync::Arc;

use tracing::{debug, info, warn};
use uuid::Uuid;

use filehub_core::config::RealtimeConfig;

use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

use super::dedup::{DedupKey, NotificationDedup};

/// Dispatches notifications to users — via WebSocket if online, or persists for offline delivery.
#[derive(Debug)]
pub struct NotificationDispatcher {
    /// Connection manager for sending to online users.
    connections: Arc<ConnectionManager>,
    /// Deduplication engine.
    dedup: NotificationDedup,
}

impl NotificationDispatcher {
    /// Creates a new notification dispatcher.
    pub fn new(connections: Arc<ConnectionManager>, config: RealtimeConfig) -> Self {
        let batch_window = config.notifications.batch_window_ms as u64;

        Self {
            connections,
            dedup: NotificationDedup::new(batch_window),
        }
    }

    /// Dispatches a notification to a single user.
    ///
    /// If the user is online, sends via WebSocket.
    /// Deduplication is applied for rapid events.
    pub async fn dispatch_to_user(
        &self,
        user_id: Uuid,
        event_type: &str,
        resource_id: Option<Uuid>,
        message: OutboundMessage,
    ) {
        // Check dedup
        let dedup_key = DedupKey {
            user_id,
            event_type: event_type.to_string(),
            resource_id,
        };

        if !self.dedup.should_deliver(dedup_key).await {
            debug!(
                user_id = %user_id,
                event_type = %event_type,
                "Notification deduplicated"
            );
            return;
        }

        // Try sending via WebSocket
        if self.connections.is_user_connected(&user_id) {
            self.connections.send_to_user(&user_id, &message).await;
            debug!(user_id = %user_id, "Notification sent via WebSocket");
        } else {
            debug!(user_id = %user_id, "User offline, notification should be persisted");
            // Persistence is handled by the caller (service layer)
        }
    }

    /// Dispatches a notification to multiple users.
    pub async fn dispatch_to_users(
        &self,
        user_ids: &[Uuid],
        event_type: &str,
        resource_id: Option<Uuid>,
        message: OutboundMessage,
    ) {
        for user_id in user_ids {
            self.dispatch_to_user(*user_id, event_type, resource_id, message.clone())
                .await;
        }
    }

    /// Dispatches a notification to a channel.
    pub async fn dispatch_to_channel(&self, channel: &str, message: OutboundMessage) {
        self.connections
            .broadcast_to_channel(channel, &message)
            .await;
    }

    /// Sends a session termination notice.
    pub async fn send_session_termination(
        &self,
        session_id: Uuid,
        reason: &str,
        terminated_by: Option<Uuid>,
    ) {
        let message = OutboundMessage::SessionTerminated {
            session_id,
            reason: reason.to_string(),
            terminated_by,
            grace_seconds: 5,
        };

        self.connections
            .send_to_session(&session_id, &message)
            .await;

        info!(
            session_id = %session_id,
            "Session termination notice sent"
        );
    }

    /// Broadcasts an admin message to all users.
    pub async fn broadcast_admin_message(
        &self,
        id: Uuid,
        title: &str,
        message_text: &str,
        severity: &str,
        persistent: bool,
    ) {
        let message = OutboundMessage::AdminBroadcast {
            id,
            title: title.to_string(),
            message: message_text.to_string(),
            severity: severity.to_string(),
            persistent,
            action: None,
            timestamp: chrono::Utc::now(),
        };

        self.connections.broadcast_all(&message).await;

        info!(
            broadcast_id = %id,
            severity = %severity,
            "Admin broadcast sent"
        );
    }

    /// Sends an upload progress update.
    pub async fn send_progress(&self, user_id: Uuid, upload_id: Uuid, percent: u8, status: &str) {
        let message = OutboundMessage::Progress {
            resource_id: upload_id,
            percent,
            status: status.to_string(),
            details: None,
        };

        self.connections.send_to_user(&user_id, &message).await;
    }

    /// Returns a reference to the connection manager.
    pub fn connections(&self) -> &Arc<ConnectionManager> {
        &self.connections
    }

    /// Cleans up dedup buffer.
    pub async fn cleanup_dedup(&self) {
        self.dedup.cleanup().await;
    }
}
