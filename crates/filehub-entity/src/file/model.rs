//! File entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A file stored in FileHub.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct File {
    /// Unique file identifier.
    pub id: Uuid,
    /// The folder containing this file.
    pub folder_id: Uuid,
    /// The storage backend where the file is physically stored.
    pub storage_id: Uuid,
    /// The file name (including extension).
    pub name: String,
    /// The path within the storage provider.
    pub storage_path: String,
    /// MIME type of the file.
    pub mime_type: Option<String>,
    /// File size in bytes.
    pub size_bytes: i64,
    /// SHA-256 checksum of the file content.
    pub checksum_sha256: Option<String>,
    /// Arbitrary metadata (JSON).
    pub metadata: Option<serde_json::Value>,
    /// Current version number.
    pub current_version: i32,
    /// Whether the file is currently locked for editing.
    pub is_locked: Option<bool>,
    /// The user who holds the lock (if locked).
    pub locked_by: Option<Uuid>,
    /// When the lock was acquired.
    pub locked_at: Option<DateTime<Utc>>,
    /// The file owner.
    pub owner_id: Uuid,
    /// When the file was created.
    pub created_at: DateTime<Utc>,
    /// When the file was last updated.
    pub updated_at: DateTime<Utc>,
}

impl File {
    /// Check if the file is currently locked.
    pub fn is_file_locked(&self) -> bool {
        self.is_locked.unwrap_or(false)
    }

    /// Get the file extension (lowercase), if any.
    pub fn extension(&self) -> Option<String> {
        self.name
            .rsplit('.')
            .next()
            .filter(|ext| *ext != self.name)
            .map(|ext| ext.to_lowercase())
    }
}

/// Data required to create a new file record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFile {
    /// The folder to place the file in.
    pub folder_id: Uuid,
    /// The storage backend.
    pub storage_id: Uuid,
    /// The file name.
    pub name: String,
    /// The path within the storage provider.
    pub storage_path: String,
    /// MIME type.
    pub mime_type: Option<String>,
    /// File size in bytes.
    pub size_bytes: i64,
    /// SHA-256 checksum.
    pub checksum_sha256: Option<String>,
    /// Arbitrary metadata.
    pub metadata: Option<serde_json::Value>,
    /// The file owner.
    pub owner_id: Uuid,
}
