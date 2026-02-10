//! Message validation rules.

use super::types::InboundMessage;
use filehub_core::error::AppError;

/// Maximum allowed message size in bytes.
const MAX_MESSAGE_SIZE: usize = 65_536;

/// Validates an inbound message.
pub fn validate_inbound(raw: &str) -> Result<(), AppError> {
    if raw.len() > MAX_MESSAGE_SIZE {
        return Err(AppError::validation(format!(
            "Message exceeds maximum size of {} bytes",
            MAX_MESSAGE_SIZE
        )));
    }

    if raw.trim().is_empty() {
        return Err(AppError::validation("Empty message"));
    }

    Ok(())
}

/// Validates channel name format.
pub fn validate_channel_name(channel: &str) -> Result<(), AppError> {
    if channel.is_empty() || channel.len() > 256 {
        return Err(AppError::validation("Invalid channel name length"));
    }

    if !channel
        .chars()
        .all(|c| c.is_alphanumeric() || c == ':' || c == '-' || c == '_')
    {
        return Err(AppError::validation(
            "Channel name contains invalid characters",
        ));
    }

    Ok(())
}
