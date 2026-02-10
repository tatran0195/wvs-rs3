//! Message envelope for framing WebSocket messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::OutboundMessage;

/// Envelope wrapping outbound messages with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message ID for deduplication and ack
    pub id: String,
    /// Channel this message was sent on (if any)
    pub channel: Option<String>,
    /// The message payload
    pub data: OutboundMessage,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Sequence number (per connection)
    pub seq: u64,
}

impl MessageEnvelope {
    /// Create a new envelope wrapping a message
    pub fn new(data: OutboundMessage, channel: Option<String>, seq: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            channel,
            data,
            timestamp: Utc::now(),
            seq,
        }
    }

    /// Create an envelope for a direct (non-channel) message
    pub fn direct(data: OutboundMessage, seq: u64) -> Self {
        Self::new(data, None, seq)
    }

    /// Create an envelope for a channel message
    pub fn on_channel(data: OutboundMessage, channel: &str, seq: u64) -> Self {
        Self::new(data, Some(channel.to_string()), seq)
    }
}
