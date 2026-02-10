//! JSON serialization for WebSocket messages.

use serde_json;

use super::envelope::MessageEnvelope;
use super::types::{InboundMessage, OutboundMessage};

/// Serialize an outbound message envelope to JSON
pub fn serialize_envelope(envelope: &MessageEnvelope) -> Result<String, serde_json::Error> {
    serde_json::to_string(envelope)
}

/// Serialize an outbound message directly (without envelope)
pub fn serialize_outbound(msg: &OutboundMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

/// Deserialize an inbound message from JSON
pub fn deserialize_inbound(text: &str) -> Result<InboundMessage, serde_json::Error> {
    serde_json::from_str(text)
}
