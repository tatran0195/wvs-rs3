//! Typed path parameter helpers.

use uuid::Uuid;

use filehub_core::error::AppError;

/// Parses a UUID from a path segment.
pub fn parse_uuid(s: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(s).map_err(|_| AppError::validation(format!("Invalid UUID: {s}")))
}
