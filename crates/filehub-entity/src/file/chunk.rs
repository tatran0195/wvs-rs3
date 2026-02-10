//! Chunked upload entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Status of a chunked upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStatus {
    /// Upload is in progress.
    Uploading,
    /// All chunks received, assembly in progress.
    Assembling,
    /// Upload completed and assembled.
    Completed,
    /// Upload failed.
    Failed,
    /// Upload was cancelled or timed out.
    Expired,
}

impl ChunkStatus {
    /// Return the status as a string for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploading => "uploading",
            Self::Assembling => "assembling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }
}

impl std::fmt::Display for ChunkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A chunked upload session tracking progress of a multi-part upload.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChunkedUpload {
    /// Unique upload session identifier.
    pub id: Uuid,
    /// The user performing the upload.
    pub user_id: Uuid,
    /// Target storage backend.
    pub storage_id: Uuid,
    /// Target folder for the completed file.
    pub target_folder_id: Uuid,
    /// The intended file name.
    pub file_name: String,
    /// Total file size in bytes.
    pub file_size: i64,
    /// MIME type (if known).
    pub mime_type: Option<String>,
    /// Size of each chunk in bytes.
    pub chunk_size: i32,
    /// Total number of chunks expected.
    pub total_chunks: i32,
    /// Array of completed chunk numbers (JSON array).
    pub uploaded_chunks: serde_json::Value,
    /// Expected SHA-256 checksum of the final assembled file.
    pub checksum_sha256: Option<String>,
    /// Temporary storage path for chunk data.
    pub temp_path: String,
    /// Current upload status.
    pub status: String,
    /// When the upload session was created.
    pub created_at: DateTime<Utc>,
    /// When the upload session expires.
    pub expires_at: DateTime<Utc>,
    /// When the upload was completed (if applicable).
    pub completed_at: Option<DateTime<Utc>>,
}

impl ChunkedUpload {
    /// Get the list of uploaded chunk numbers.
    pub fn uploaded_chunk_numbers(&self) -> Vec<i32> {
        serde_json::from_value(self.uploaded_chunks.clone()).unwrap_or_default()
    }

    /// Get the number of chunks that have been uploaded.
    pub fn uploaded_count(&self) -> usize {
        self.uploaded_chunk_numbers().len()
    }

    /// Check if all chunks have been uploaded.
    pub fn is_complete(&self) -> bool {
        self.uploaded_count() as i32 >= self.total_chunks
    }

    /// Calculate the upload progress as a percentage (0-100).
    pub fn progress_percent(&self) -> f64 {
        if self.total_chunks <= 0 {
            return 0.0;
        }
        (self.uploaded_count() as f64 / self.total_chunks as f64) * 100.0
    }
}
