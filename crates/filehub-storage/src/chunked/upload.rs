//! Chunked upload handler for multi-part file uploads.

use std::sync::Arc;

use bytes::Bytes;
use uuid::Uuid;

use filehub_core::result::AppResult;
use filehub_core::traits::storage::StorageProvider;

/// Handles individual chunk writes during a chunked upload.
#[derive(Debug, Clone)]
pub struct ChunkedUploadHandler {
    /// The storage provider for temporary chunk storage.
    provider: Arc<dyn StorageProvider>,
}

impl ChunkedUploadHandler {
    /// Create a new chunked upload handler.
    pub fn new(provider: Arc<dyn StorageProvider>) -> Self {
        Self { provider }
    }

    /// Write a single chunk to temporary storage.
    pub async fn write_chunk(
        &self,
        upload_id: Uuid,
        chunk_number: i32,
        data: Bytes,
    ) -> AppResult<u64> {
        let chunk_path = Self::chunk_path(upload_id, chunk_number);
        let size = data.len() as u64;
        self.provider.write(&chunk_path, data).await?;
        Ok(size)
    }

    /// Read a chunk from temporary storage.
    pub async fn read_chunk(&self, upload_id: Uuid, chunk_number: i32) -> AppResult<Bytes> {
        let chunk_path = Self::chunk_path(upload_id, chunk_number);
        self.provider.read_bytes(&chunk_path).await
    }

    /// Delete a single chunk.
    pub async fn delete_chunk(&self, upload_id: Uuid, chunk_number: i32) -> AppResult<()> {
        let chunk_path = Self::chunk_path(upload_id, chunk_number);
        self.provider.delete(&chunk_path).await
    }

    /// Delete all chunks for an upload.
    pub async fn delete_all_chunks(&self, upload_id: Uuid, total_chunks: i32) -> AppResult<()> {
        for i in 0..total_chunks {
            let chunk_path = Self::chunk_path(upload_id, i);
            // Ignore errors for chunks that may not exist.
            let _ = self.provider.delete(&chunk_path).await;
        }
        // Also try to remove the upload directory.
        let dir_path = Self::upload_dir(upload_id);
        let _ = self.provider.delete_dir(&dir_path).await;
        Ok(())
    }

    /// Check if a specific chunk exists.
    pub async fn chunk_exists(&self, upload_id: Uuid, chunk_number: i32) -> AppResult<bool> {
        let chunk_path = Self::chunk_path(upload_id, chunk_number);
        self.provider.exists(&chunk_path).await
    }

    /// Generate the temporary storage path for a chunk.
    pub fn chunk_path(upload_id: Uuid, chunk_number: i32) -> String {
        format!("_chunks/{upload_id}/{chunk_number:06}")
    }

    /// Generate the temporary directory for an upload.
    pub fn upload_dir(upload_id: Uuid) -> String {
        format!("_chunks/{upload_id}")
    }
}
