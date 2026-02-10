//! Chunk assembler — concatenates chunks into a final file.

use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::traits::storage::StorageProvider;

use super::upload::ChunkedUploadHandler;

/// Assembles uploaded chunks into a final file.
#[derive(Debug, Clone)]
pub struct ChunkAssembler {
    /// Handler for reading chunks.
    upload_handler: ChunkedUploadHandler,
    /// Target storage provider.
    target_provider: Arc<dyn StorageProvider>,
}

impl ChunkAssembler {
    /// Create a new chunk assembler.
    pub fn new(
        upload_handler: ChunkedUploadHandler,
        target_provider: Arc<dyn StorageProvider>,
    ) -> Self {
        Self {
            upload_handler,
            target_provider,
        }
    }

    /// Assemble all chunks into a single file at the target path.
    ///
    /// Reads each chunk in order and writes them sequentially to the
    /// target storage. Returns the total number of bytes written.
    pub async fn assemble(
        &self,
        upload_id: Uuid,
        total_chunks: i32,
        target_path: &str,
    ) -> AppResult<u64> {
        tracing::info!(
            upload_id = %upload_id,
            total_chunks,
            target_path,
            "Assembling chunks"
        );

        // Create a temporary local file to assemble into, then write to target.
        let temp_dir = std::env::temp_dir().join(format!("filehub_assemble_{upload_id}"));
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| {
            AppError::with_source(ErrorKind::Storage, "Failed to create temp assembly dir", e)
        })?;

        let temp_file_path = temp_dir.join("assembled");
        let mut file = tokio::fs::File::create(&temp_file_path)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Storage, "Failed to create temp assembly file", e)
            })?;

        let mut total_bytes = 0u64;

        for chunk_num in 0..total_chunks {
            let chunk_data = self.upload_handler.read_chunk(upload_id, chunk_num).await?;
            total_bytes += chunk_data.len() as u64;
            file.write_all(&chunk_data).await.map_err(|e| {
                AppError::with_source(ErrorKind::Storage, "Failed to write chunk to assembly", e)
            })?;
        }

        file.flush().await.map_err(|e| {
            AppError::with_source(ErrorKind::Storage, "Failed to flush assembly file", e)
        })?;
        drop(file);

        // Read the assembled file and write to target storage.
        let assembled_bytes = tokio::fs::read(&temp_file_path).await.map_err(|e| {
            AppError::with_source(ErrorKind::Storage, "Failed to read assembled file", e)
        })?;

        self.target_provider
            .write(target_path, bytes::Bytes::from(assembled_bytes))
            .await?;

        // Clean up temp directory.
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        // Clean up chunks.
        self.upload_handler
            .delete_all_chunks(upload_id, total_chunks)
            .await?;

        tracing::info!(
            upload_id = %upload_id,
            bytes = total_bytes,
            "Assembly complete"
        );

        Ok(total_bytes)
    }
}
