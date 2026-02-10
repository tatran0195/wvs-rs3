//! Message envelope wrapping for routing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Envelope wrapping a message with routing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique envelope ID.
    pub id: Uuid,
    /// Target channel (if channel-targeted).
    pub channel: Option<String>,
    /// Target user ID (if user-targeted).
    pub user_id: Option<Uuid>,
    /// Target session ID (if session-targeted).
    pub session_id: Option<Uuid>,
    /// The serialized message payload.
    pub payload: String,
    /// When the envelope was created.
    pub created_at: DateTime<Utc>,
}

impl MessageEnvelope {
    /// Creates a channel-targeted envelope.
    pub fn for_channel(channel: &str, payload: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel: Some(channel.to_string()),
            user_id: None,
            session_id: None,
            payload,
            created_at: Utc::now(),
        }
    }

    /// Creates a user-targeted envelope.
    pub fn for_user(user_id: Uuid, payload: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel: None,
            user_id: Some(user_id),
            session_id: None,
            payload,
            created_at: Utc::now(),
        }
    }

    /// Creates a session-targeted envelope.
    pub fn for_session(session_id: Uuid, payload: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel: None,
            user_id: None,
            session_id: Some(session_id),
            payload,
            created_at: Utc::now(),
        }
    }
}
