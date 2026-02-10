//! Inbound and outbound WebSocket message type definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent by the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMessage {
    /// Subscribe to a channel.
    Subscribe {
        /// Channel name.
        channel: String,
    },
    /// Unsubscribe from a channel.
    Unsubscribe {
        /// Channel name.
        channel: String,
    },
    /// Pong response to server ping.
    Pong {
        /// Echoed timestamp.
        timestamp: i64,
    },
    /// Update user presence status.
    PresenceUpdate {
        /// New status.
        status: String,
    },
    /// Mark a notification as read.
    MarkRead {
        /// Notification ID.
        notification_id: Uuid,
    },
    /// Acknowledge receipt of a message.
    Ack {
        /// Message ID being acknowledged.
        message_id: Uuid,
    },
}

/// Messages sent by the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMessage {
    /// Subscription confirmed.
    Subscribed {
        /// Channel name.
        channel: String,
    },
    /// Notification delivery.
    Notification {
        /// Unique message ID.
        id: Uuid,
        /// Notification category.
        category: String,
        /// Event type.
        event_type: String,
        /// Notification title.
        title: String,
        /// Notification body.
        message: String,
        /// Additional payload.
        payload: Option<serde_json::Value>,
        /// Priority level.
        priority: String,
        /// Timestamp.
        timestamp: DateTime<Utc>,
    },
    /// Upload progress update.
    Progress {
        /// Upload or job ID.
        resource_id: Uuid,
        /// Progress percentage (0-100).
        percent: u8,
        /// Status message.
        status: String,
        /// Additional details.
        details: Option<serde_json::Value>,
    },
    /// Session terminated notification.
    SessionTerminated {
        /// Session ID.
        session_id: Uuid,
        /// Reason for termination.
        reason: String,
        /// Who terminated (admin ID).
        terminated_by: Option<Uuid>,
        /// Grace period in seconds before forced close.
        grace_seconds: u32,
    },
    /// Admin broadcast message.
    AdminBroadcast {
        /// Broadcast ID.
        id: Uuid,
        /// Title.
        title: String,
        /// Message body.
        message: String,
        /// Severity level.
        severity: String,
        /// Whether the message should persist.
        persistent: bool,
        /// Optional action.
        action: Option<BroadcastAction>,
        /// Timestamp.
        timestamp: DateTime<Utc>,
    },
    /// Presence update for another user.
    PresenceChange {
        /// User ID.
        user_id: Uuid,
        /// Username.
        username: String,
        /// New status.
        status: String,
        /// Timestamp.
        timestamp: DateTime<Utc>,
    },
    /// Ping (server keepalive).
    Ping {
        /// Server timestamp.
        timestamp: i64,
    },
    /// Error message.
    Error {
        /// Error code.
        code: String,
        /// Error description.
        message: String,
    },
}

/// Action associated with an admin broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastAction {
    /// Action type (e.g., "force_logout", "reload").
    pub action_type: String,
    /// Action payload.
    pub payload: Option<serde_json::Value>,
}
