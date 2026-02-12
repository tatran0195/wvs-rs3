//! Admin session monitor — provides real-time session view.

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::connection::handle::ConnectionInfo;
use crate::connection::manager::ConnectionManager;
use crate::message::types::OutboundMessage;

/// Admin session monitor
#[derive(Debug)]
pub struct SessionMonitor {
    /// Connection manager
    connections: Arc<ConnectionManager>,
}

impl SessionMonitor {
    /// Create a new session monitor
    pub fn new(connections: Arc<ConnectionManager>) -> Self {
        Self { connections }
    }

    /// Get all active connection info for admin view
    pub async fn get_live_sessions(&self) -> Vec<ConnectionInfo> {
        self.connections.all_connection_info().await
    }

    /// Get real-time stats
    pub fn get_stats(&self) -> SessionStats {
        SessionStats {
            total_connections: self.connections.total_connections(),
            unique_users: self.connections.unique_users(),
            timestamp: Utc::now(),
        }
    }

    /// Emit session count update to admin channel
    pub fn build_count_update(&self, total_seats: i32, available: i32) -> OutboundMessage {
        OutboundMessage::SessionCountUpdated {
            active_sessions: self.connections.total_connections() as i32,
            total_seats,
            available_seats: available,
            timestamp: Utc::now(),
        }
    }
}

/// Real-time session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total WebSocket connections
    pub total_connections: usize,
    /// Unique connected users
    pub unique_users: usize,
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
}
