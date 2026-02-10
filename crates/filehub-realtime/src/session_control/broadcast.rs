//! Admin broadcast message sending.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::notification::AdminBroadcastRepository;
use filehub_entity::notification::AdminBroadcast;

use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

/// Sends admin broadcast messages to all connected users.
#[derive(Debug)]
pub struct AdminBroadcaster {
    /// Connection manager.
    connections: Arc<ConnectionManager>,
    /// Broadcast repository.
    broadcast_repo: Arc<AdminBroadcastRepository>,
}

impl AdminBroadcaster {
    /// Creates a new admin broadcaster.
    pub fn new(
        connections: Arc<ConnectionManager>,
        broadcast_repo: Arc<AdminBroadcastRepository>,
    ) -> Self {
        Self {
            connections,
            broadcast_repo,
        }
    }

    /// Sends a broadcast to all connected users and persists it.
    pub async fn send_broadcast(
        &self,
        admin_id: Uuid,
        target: &str,
        title: &str,
        message: &str,
        severity: &str,
        persistent: bool,
    ) -> Result<AdminBroadcast, AppError> {
        let broadcast = AdminBroadcast {
            id: Uuid::new_v4(),
            admin_id,
            target: target.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            persistent,
            action_type: None,
            action_payload: None,
            delivered_count: self.connections.connection_count() as i32,
            created_at: Utc::now(),
        };

        // Persist
        self.broadcast_repo
            .create(&broadcast)
            .await
            .map_err(|e| AppError::internal(format!("Failed to persist broadcast: {e}")))?;

        // Send via WebSocket
        let ws_message = OutboundMessage::AdminBroadcast {
            id: broadcast.id,
            title: title.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            persistent,
            action: None,
            timestamp: Utc::now(),
        };

        match target {
            "all" => {
                self.connections.broadcast_all(&ws_message).await;
            }
            _ => {
                self.connections
                    .broadcast_to_channel(target, &ws_message)
                    .await;
            }
        }

        info!(
            broadcast_id = %broadcast.id,
            admin_id = %admin_id,
            target = %target,
            severity = %severity,
            "Admin broadcast sent"
        );

        Ok(broadcast)
    }
}
