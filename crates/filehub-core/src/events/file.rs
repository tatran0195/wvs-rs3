//! File-related domain events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to file operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileEvent {
    /// A file was uploaded.
    Uploaded {
        /// The file ID.
        file_id: Uuid,
        /// The folder containing the file.
        folder_id: Uuid,
        /// The storage backend used.
        storage_id: Uuid,
        /// The file name.
        name: String,
        /// The file size in bytes.
        size_bytes: u64,
        /// The MIME type (if known).
        mime_type: Option<String>,
    },
    /// A file was downloaded.
    Downloaded {
        /// The file ID.
        file_id: Uuid,
        /// The user who downloaded it.
        user_id: Uuid,
    },
    /// A file was updated (metadata or content).
    Updated {
        /// The file ID.
        file_id: Uuid,
        /// Fields that changed.
        changed_fields: Vec<String>,
    },
    /// A file was deleted.
    Deleted {
        /// The file ID.
        file_id: Uuid,
        /// The file name (for display after deletion).
        name: String,
        /// The folder it was in.
        folder_id: Uuid,
    },
    /// A file was moved.
    Moved {
        /// The file ID.
        file_id: Uuid,
        /// The source folder.
        from_folder_id: Uuid,
        /// The destination folder.
        to_folder_id: Uuid,
    },
    /// A file was copied.
    Copied {
        /// The original file ID.
        source_file_id: Uuid,
        /// The new copy file ID.
        new_file_id: Uuid,
        /// The destination folder.
        to_folder_id: Uuid,
    },
    /// A file was locked.
    Locked {
        /// The file ID.
        file_id: Uuid,
        /// The user who locked it.
        locked_by: Uuid,
    },
    /// A file was unlocked.
    Unlocked {
        /// The file ID.
        file_id: Uuid,
        /// The user who unlocked it.
        unlocked_by: Uuid,
    },
    /// A new file version was created.
    VersionCreated {
        /// The file ID.
        file_id: Uuid,
        /// The new version number.
        version_number: i32,
    },
}
