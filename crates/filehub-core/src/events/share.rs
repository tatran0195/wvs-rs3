//! Share-related domain events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to sharing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShareEvent {
    /// A share was created.
    Created {
        /// The share ID.
        share_id: Uuid,
        /// The resource type being shared.
        resource_type: String,
        /// The resource ID.
        resource_id: Uuid,
        /// The share type (public_link, private_link, user_share).
        share_type: String,
    },
    /// A share was accessed.
    Accessed {
        /// The share ID.
        share_id: Uuid,
        /// The accessor's IP address (if available).
        ip_address: Option<String>,
    },
    /// A share was revoked.
    Revoked {
        /// The share ID.
        share_id: Uuid,
        /// The resource ID.
        resource_id: Uuid,
    },
    /// A shared file was downloaded.
    Downloaded {
        /// The share ID.
        share_id: Uuid,
        /// Current download count.
        download_count: i32,
        /// Maximum downloads allowed (if set).
        max_downloads: Option<i32>,
    },
    /// A share expired.
    Expired {
        /// The share ID.
        share_id: Uuid,
    },
}
