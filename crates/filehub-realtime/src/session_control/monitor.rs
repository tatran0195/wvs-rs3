//! Admin real-time session monitoring.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::channel::registry::ChannelRegistry;
use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

/// Session snapshot for admin monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Total active connections.
    pub total_connections: usize,
    /// Unique connected users.
    pub unique_users: usize,
    /// Active channel count.
    pub active_channels: usize,
}

/// Provides real-time session monitoring for admins.
#[derive(Debug)]
pub struct SessionMonitor {
    /// Connection manager.
    connections: Arc<ConnectionManager>,
    /// Channel registry.
    channels: Arc<ChannelRegistry>,
}

impl SessionMonitor {
    /// Creates a new session monitor.
    pub fn new(connections: Arc<ConnectionManager>, channels: Arc<ChannelRegistry>) -> Self {
        Self {
            connections,
            channels,
        }
    }

    /// Takes a snapshot of the current session state.
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            total_connections: self.connections.connection_count(),
            unique_users: self.connections.user_count(),
            active_channels: self.channels.channel_count(),
        }
    }

    /// Pushes a session update to admin subscribers.
    pub async fn push_update(&self) {
        let snapshot = self.snapshot();

        let message = OutboundMessage::Notification {
            id: Uuid::new_v4(),
            category: "admin".to_string(),
            event_type: "session.snapshot".to_string(),
            title: "Session Update".to_string(),
            message: format!(
                "{} connections, {} users",
                snapshot.total_connections, snapshot.unique_users
            ),
            payload: Some(serde_json::to_value(&snapshot).unwrap_or_default()),
            priority: "low".to_string(),
            timestamp: chrono::Utc::now(),
        };

        self.connections
            .broadcast_to_channel("admin:sessions", &message)
            .await;
    }
}
