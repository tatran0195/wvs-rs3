//! Typed job payload definitions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Typed payloads for known job types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "job_type")]
pub enum JobPayload {
    /// Assemble chunks into a final file.
    #[serde(rename = "file_assembly")]
    FileAssembly {
        /// Chunked upload ID.
        upload_id: Uuid,
        /// Target folder ID.
        target_folder_id: Uuid,
        /// Storage ID.
        storage_id: Uuid,
    },
    /// Convert a CAD file.
    #[serde(rename = "cad_conversion")]
    CadConversion {
        /// File ID to convert.
        file_id: Uuid,
        /// Source format.
        source_format: String,
        /// Target format.
        target_format: String,
    },
    /// Generate a thumbnail.
    #[serde(rename = "thumbnail_generation")]
    ThumbnailGeneration {
        /// File ID.
        file_id: Uuid,
        /// Requested sizes.
        sizes: Vec<u32>,
    },
    /// Clean up expired sessions.
    #[serde(rename = "session_cleanup")]
    SessionCleanup,
    /// Clean up expired chunks.
    #[serde(rename = "chunk_cleanup")]
    ChunkCleanup,
    /// Clean up temporary files.
    #[serde(rename = "temp_cleanup")]
    TempCleanup,
    /// Generate a weekly report.
    #[serde(rename = "weekly_report")]
    WeeklyReport,
    /// Sync license pool with FlexNet.
    #[serde(rename = "license_pool_sync")]
    LicensePoolSync,
    /// Reconcile presence data.
    #[serde(rename = "presence_reconciliation")]
    PresenceReconciliation,
    /// Clean up old notifications.
    #[serde(rename = "notification_cleanup")]
    NotificationCleanup,
    /// Cross-storage file transfer.
    #[serde(rename = "storage_transfer")]
    StorageTransfer {
        /// File ID.
        file_id: Uuid,
        /// Source storage.
        source_storage_id: Uuid,
        /// Destination storage.
        target_storage_id: Uuid,
    },
}
