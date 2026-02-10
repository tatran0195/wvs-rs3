//! Orphan chunk cleanup.

use std::sync::Arc;

use filehub_core::result::AppResult;
use filehub_core::traits::storage::StorageProvider;

/// Cleans up orphaned chunk files from expired/failed uploads.
#[derive(Debug, Clone)]
pub struct OrphanChunkCleanup {
    /// Storage provider where chunks are stored.
    provider: Arc<dyn StorageProvider>,
}

impl OrphanChunkCleanup {
    /// Create a new orphan chunk cleanup handler.
    pub fn new(provider: Arc<dyn StorageProvider>) -> Self {
        Self { provider }
    }

    /// Delete the chunk directory for a specific upload.
    pub async fn cleanup_upload(&self, upload_id: uuid::Uuid) -> AppResult<()> {
        let dir = format!("_chunks/{upload_id}");
        self.provider.delete_dir(&dir).await?;
        tracing::debug!(upload_id = %upload_id, "Cleaned up orphan chunks");
        Ok(())
    }

    /// List all upload directories in the chunks area.
    pub async fn list_upload_dirs(&self) -> AppResult<Vec<String>> {
        let entries = self.provider.list("_chunks").await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.is_directory)
            .map(|e| e.path)
            .collect())
    }
}
