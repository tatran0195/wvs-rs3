//! JSON serialization helpers for WebSocket messages.

use super::types::OutboundMessage;
use filehub_core::error::AppError;

/// Serializes an outbound message to JSON.
pub fn serialize(msg: &OutboundMessage) -> Result<String, AppError> {
    serde_json::to_string(msg)
        .map_err(|e| AppError::internal(format!("Message serialization failed: {e}")))
}

/// Deserializes an inbound message from JSON.
pub fn deserialize_inbound(raw: &str) -> Result<super::types::InboundMessage, AppError> {
    serde_json::from_str(raw)
        .map_err(|e| AppError::validation(format!("Invalid message format: {e}")))
}
