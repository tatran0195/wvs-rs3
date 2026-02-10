//! Share entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::permission::acl::AclPermission;
use crate::permission::model::ResourceType;

/// Type of share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "share_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ShareType {
    /// A publicly accessible link (no authentication required).
    PublicLink,
    /// A link that requires a password.
    PrivateLink,
    /// A share directly with another user.
    UserShare,
}

/// A share granting access to a file or folder.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Share {
    /// Unique share identifier.
    pub id: Uuid,
    /// Type of share.
    pub share_type: ShareType,
    /// Type of resource being shared.
    pub resource_type: ResourceType,
    /// ID of the shared resource.
    pub resource_id: Uuid,
    /// User who created the share.
    pub created_by: Uuid,
    /// Share token for link-based sharing.
    pub token: Option<String>,
    /// Password hash for private links.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    /// User this is shared with (for user_share type).
    pub shared_with: Option<Uuid>,
    /// Permission level granted.
    pub permission: AclPermission,
    /// Whether download is allowed.
    pub allow_download: Option<bool>,
    /// Maximum number of downloads.
    pub max_downloads: Option<i32>,
    /// Current download count.
    pub download_count: Option<i32>,
    /// When the share expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the share is currently active.
    pub is_active: Option<bool>,
    /// When the share was created.
    pub created_at: DateTime<Utc>,
    /// Last time the share was accessed.
    pub last_accessed: Option<DateTime<Utc>>,
}

impl Share {
    /// Check if the share is currently valid.
    pub fn is_valid(&self) -> bool {
        if !self.is_active.unwrap_or(true) {
            return false;
        }
        if let Some(expires_at) = self.expires_at {
            if expires_at <= Utc::now() {
                return false;
            }
        }
        if let (Some(max), Some(count)) = (self.max_downloads, self.download_count) {
            if count >= max {
                return false;
            }
        }
        true
    }

    /// Check if downloads are allowed.
    pub fn downloads_allowed(&self) -> bool {
        self.allow_download.unwrap_or(true)
    }
}

/// Data required to create a new share.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShare {
    /// Type of share.
    pub share_type: ShareType,
    /// Type of resource.
    pub resource_type: ResourceType,
    /// ID of the resource.
    pub resource_id: Uuid,
    /// User creating the share.
    pub created_by: Uuid,
    /// Share token (for links).
    pub token: Option<String>,
    /// Password hash (for private links).
    pub password_hash: Option<String>,
    /// User being shared with (for user_share).
    pub shared_with: Option<Uuid>,
    /// Permission level.
    pub permission: AclPermission,
    /// Allow downloads.
    pub allow_download: bool,
    /// Max downloads (None = unlimited).
    pub max_downloads: Option<i32>,
    /// Expiry time (None = never).
    pub expires_at: Option<DateTime<Utc>>,
}
