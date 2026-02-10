//! Cross-storage file transfer.

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::result::AppResult;

use crate::manager::StorageManager;

/// Service for transferring files between storage providers.
#[derive(Debug, Clone)]
pub struct CrossStorageTransfer {
    /// Storage manager to resolve providers.
    storage_manager: StorageManager,
}

impl CrossStorageTransfer {
    /// Create a new cross-storage transfer service.
    pub fn new(storage_manager: StorageManager) -> Self {
        Self { storage_manager }
    }

    /// Transfer a file from one storage to another.
    ///
    /// Reads the file from the source provider and writes it to the
    /// destination provider, then optionally deletes from source.
    pub async fn transfer(
        &self,
        source_storage_id: &Uuid,
        source_path: &str,
        target_storage_id: &Uuid,
        target_path: &str,
        delete_source: bool,
    ) -> AppResult<u64> {
        if source_storage_id == target_storage_id {
            return Err(AppError::validation(
                "Source and target storage must be different for transfer",
            ));
        }

        let source = self.storage_manager.get(source_storage_id).await?;
        let target = self.storage_manager.get(target_storage_id).await?;

        // Stream from source to target.
        let stream = source.read(source_path).await?;
        let bytes_written = target.write_stream(target_path, stream).await?;

        if delete_source {
            source.delete(source_path).await?;
        }

        tracing::info!(
            source_storage = %source_storage_id,
            target_storage = %target_storage_id,
            source_path,
            target_path,
            bytes = bytes_written,
            "Completed cross-storage transfer"
        );

        Ok(bytes_written)
    }
}
