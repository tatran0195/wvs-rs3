//! Domain event → notification mapping.
//!
//! Bridges domain events from the service layer to the
//! notification and channel system.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::channel::types::ChannelType;
use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;
use crate::notification::dispatcher::NotificationDispatcher;
use crate::presence::tracker::PresenceTracker;

/// Bridges domain events into the realtime system.
#[derive(Debug)]
pub struct EventBridge {
    /// Connection manager
    connections: Arc<ConnectionManager>,
    /// Notification dispatcher
    notifications: Arc<NotificationDispatcher>,
    /// Presence tracker
    presence: Arc<PresenceTracker>,
}

impl EventBridge {
    /// Create a new event bridge
    pub fn new(
        connections: Arc<ConnectionManager>,
        notifications: Arc<NotificationDispatcher>,
        presence: Arc<PresenceTracker>,
    ) -> Self {
        Self {
            connections,
            notifications,
            presence,
        }
    }

    /// Handle a file created event
    pub async fn on_file_created(
        &self,
        file_id: Uuid,
        file_name: &str,
        folder_id: Uuid,
        actor_id: Uuid,
        actor_name: &str,
        size_bytes: i64,
        mime_type: Option<String>,
    ) {
        let channel = ChannelType::Folder(folder_id).to_channel_name();
        let msg = OutboundMessage::FileCreated {
            file_id,
            file_name: file_name.to_string(),
            folder_id,
            actor_id,
            actor_name: actor_name.to_string(),
            size_bytes,
            mime_type,
            timestamp: Utc::now(),
        };
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle a file deleted event
    pub async fn on_file_deleted(
        &self,
        file_id: Uuid,
        file_name: &str,
        folder_id: Uuid,
        actor_id: Uuid,
        actor_name: &str,
    ) {
        let channel = ChannelType::Folder(folder_id).to_channel_name();
        let msg = OutboundMessage::FileDeleted {
            file_id,
            file_name: file_name.to_string(),
            folder_id,
            actor_id,
            actor_name: actor_name.to_string(),
            timestamp: Utc::now(),
        };
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle a session created event (admin channel)
    pub async fn on_session_created(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        username: &str,
        ip_address: &str,
        role: &str,
    ) {
        let channel = ChannelType::AdminSessions.to_channel_name();
        let msg = OutboundMessage::SessionCreated {
            session_id: filehub_core::types::id::SessionId::from(session_id),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.to_string(),
            role: role.to_string(),
            timestamp: Utc::now(),
        };
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle an upload progress event
    pub async fn on_upload_progress(
        &self,
        upload_id: Uuid,
        file_name: &str,
        chunk_number: i32,
        total_chunks: i32,
        bytes_uploaded: i64,
        total_bytes: i64,
        user_id: Uuid,
    ) {
        let channel = ChannelType::Upload(upload_id).to_channel_name();
        let msg = crate::message::builder::build_upload_progress(
            upload_id,
            file_name,
            chunk_number,
            total_chunks,
            bytes_uploaded,
            total_bytes,
        );
        // Send to the upload channel (only the uploader subscribes)
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle a user coming online
    pub async fn on_user_online(&self, user_id: Uuid, username: &str) {
        let msg = self.presence.set_online(user_id, username);
        let channel = ChannelType::PresenceGlobal.to_channel_name();
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle a user going offline
    pub async fn on_user_offline(&self, user_id: Uuid) {
        let msg = self.presence.set_offline(user_id);
        let channel = ChannelType::PresenceGlobal.to_channel_name();
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }

    /// Handle admin broadcast
    pub async fn on_admin_broadcast(
        &self,
        broadcast_id: Uuid,
        title: &str,
        message: &str,
        severity: &str,
        persistent: bool,
    ) {
        let msg = crate::message::builder::build_admin_broadcast(
            broadcast_id,
            title,
            message,
            severity,
            persistent,
        );
        self.notifications.broadcast(msg).await;
    }

    /// Handle pool status update (admin channel)
    pub async fn on_pool_status_updated(
        &self,
        total_seats: i32,
        checked_out: i32,
        available: i32,
        drift_detected: bool,
    ) {
        let channel = ChannelType::AdminSystem.to_channel_name();
        let msg = OutboundMessage::PoolStatusUpdated {
            total_seats,
            checked_out,
            available,
            drift_detected,
            timestamp: Utc::now(),
        };
        self.notifications.dispatch_to_channel(&channel, msg).await;
    }
}
