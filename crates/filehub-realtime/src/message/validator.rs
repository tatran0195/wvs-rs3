//! Message validation for inbound WebSocket messages.

use super::types::InboundMessage;
use crate::channel::types::ChannelType;

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,
}

/// Validate an inbound message
pub fn validate_inbound(msg: &InboundMessage) -> Result<(), ValidationError> {
    match msg {
        InboundMessage::Subscribe { channel } => {
            if channel.is_empty() {
                return Err(ValidationError {
                    message: "Channel name cannot be empty".to_string(),
                });
            }
            if ChannelType::parse(channel).is_none() {
                return Err(ValidationError {
                    message: format!("Invalid channel format: '{}'", channel),
                });
            }
            Ok(())
        }
        InboundMessage::Unsubscribe { channel } => {
            if channel.is_empty() {
                return Err(ValidationError {
                    message: "Channel name cannot be empty".to_string(),
                });
            }
            Ok(())
        }
        InboundMessage::PresenceUpdate { status } => {
            let valid = ["active", "idle", "away", "dnd"];
            if !valid.contains(&status.as_str()) {
                return Err(ValidationError {
                    message: format!("Invalid presence status '{}'. Valid: {:?}", status, valid),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
