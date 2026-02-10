//! File version entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A historical version of a file.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileVersion {
    /// Unique version identifier.
    pub id: Uuid,
    /// The file this version belongs to.
    pub file_id: Uuid,
    /// Sequential version number.
    pub version_number: i32,
    /// Path to this version's content in storage.
    pub storage_path: String,
    /// Size in bytes.
    pub size_bytes: i64,
    /// SHA-256 checksum.
    pub checksum_sha256: Option<String>,
    /// User who created this version.
    pub created_by: Uuid,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
    /// Optional comment describing the change.
    pub comment: Option<String>,
}
