//! Share link value object.

use serde::{Deserialize, Serialize};

/// A generated share link for external access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    /// The share ID.
    pub share_id: uuid::Uuid,
    /// The full URL for accessing the share.
    pub url: String,
    /// The share token.
    pub token: String,
    /// Whether the link is password-protected.
    pub is_password_protected: bool,
    /// When the link expires (if set).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
