//! Folder entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A folder in the file hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    /// Unique folder identifier.
    pub id: Uuid,
    /// The storage backend this folder resides on.
    pub storage_id: Uuid,
    /// Parent folder ID (null for root folders).
    pub parent_id: Option<Uuid>,
    /// Folder name.
    pub name: String,
    /// Full materialized path (e.g., `/documents/reports`).
    pub path: String,
    /// Depth in the folder tree (0 for root).
    pub depth: i32,
    /// The folder owner.
    pub owner_id: Uuid,
    /// When the folder was created.
    pub created_at: DateTime<Utc>,
    /// When the folder was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Folder {
    /// Check if this is a root folder (no parent).
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Data required to create a new folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFolder {
    /// The storage backend.
    pub storage_id: Uuid,
    /// Parent folder (None for root).
    pub parent_id: Option<Uuid>,
    /// Folder name.
    pub name: String,
    /// Full materialized path.
    pub path: String,
    /// Depth in the tree.
    pub depth: i32,
    /// The folder owner.
    pub owner_id: Uuid,
}
