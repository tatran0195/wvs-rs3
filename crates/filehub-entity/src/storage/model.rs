//! Storage entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::provider::StorageProviderType;

/// Status of a storage backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storage_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StorageStatus {
    /// Storage is active and operational.
    Active,
    /// Storage is deactivated.
    Inactive,
    /// Storage has an error.
    Error,
    /// Storage is currently syncing.
    Syncing,
}

/// A configured storage backend.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Storage {
    /// Unique storage identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Description of this storage.
    pub description: Option<String>,
    /// The provider type.
    pub provider_type: StorageProviderType,
    /// Provider-specific configuration (JSON, encrypted at rest).
    pub config: serde_json::Value,
    /// Current status.
    pub status: StorageStatus,
    /// Whether this is the default storage for new uploads.
    pub is_default: Option<bool>,
    /// Quota in bytes (NULL = unlimited).
    pub quota_bytes: Option<i64>,
    /// Used bytes.
    pub used_bytes: Option<i64>,
    /// Virtual mount path.
    pub mount_path: Option<String>,
    /// When the storage was created.
    pub created_at: DateTime<Utc>,
    /// When the storage was last updated.
    pub updated_at: DateTime<Utc>,
    /// Last sync time.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// Admin who created this storage.
    pub created_by: Option<Uuid>,
}

impl Storage {
    /// Check if this storage is the default.
    pub fn is_default_storage(&self) -> bool {
        self.is_default.unwrap_or(false)
    }

    /// Check if the storage is active.
    pub fn is_active(&self) -> bool {
        self.status == StorageStatus::Active
    }
}

/// Data required to create a new storage backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStorage {
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Provider type.
    pub provider_type: StorageProviderType,
    /// Provider-specific config (JSON).
    pub config: serde_json::Value,
    /// Whether this is the default storage.
    pub is_default: bool,
    /// Quota in bytes (None = unlimited).
    pub quota_bytes: Option<i64>,
    /// Virtual mount path.
    pub mount_path: Option<String>,
    /// Admin creating this storage.
    pub created_by: Option<Uuid>,
}
