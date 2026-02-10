//! Thumbnail generator for image files.

use std::sync::Arc;

use bytes::Bytes;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::traits::storage::StorageProvider;

/// Generates thumbnails for image files.
#[derive(Debug, Clone)]
pub struct ThumbnailGenerator {
    /// Storage provider for reading source files and writing thumbnails.
    provider: Arc<dyn StorageProvider>,
    /// Thumbnail output directory path.
    output_dir: String,
}

impl ThumbnailGenerator {
    /// Create a new thumbnail generator.
    pub fn new(provider: Arc<dyn StorageProvider>, output_dir: &str) -> Self {
        Self {
            provider,
            output_dir: output_dir.to_string(),
        }
    }

    /// Check if a file is a supported image format for thumbnailing.
    pub fn is_supported(mime_type: &str) -> bool {
        matches!(
            mime_type,
            "image/jpeg" | "image/png" | "image/gif" | "image/webp" | "image/bmp"
        )
    }

    /// Generate a thumbnail of the specified size.
    ///
    /// Returns the storage path of the generated thumbnail.
    pub async fn generate(
        &self,
        source_path: &str,
        file_id: uuid::Uuid,
        size: u32,
    ) -> AppResult<String> {
        let source_bytes = self.provider.read_bytes(source_path).await?;

        let thumbnail_bytes =
            tokio::task::spawn_blocking(move || Self::resize_image(&source_bytes, size))
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Internal, "Thumbnail task panicked", e)
                })??;

        let thumb_path = format!("{}/{}/{}x{}.jpg", self.output_dir, file_id, size, size);

        self.provider.write(&thumb_path, thumbnail_bytes).await?;

        tracing::debug!(
            source = source_path,
            size,
            output = %thumb_path,
            "Generated thumbnail"
        );

        Ok(thumb_path)
    }

    /// Resize an image to fit within the specified dimensions.
    fn resize_image(data: &[u8], max_size: u32) -> AppResult<Bytes> {
        // Basic implementation without the image crate dependency.
        // In production, this would use the `image` crate for proper resizing.
        // For now, we return the original image data to allow the system to compile
        // and function, with proper resizing to be added when the image crate is
        // integrated.
        let _ = max_size;
        if data.is_empty() {
            return Err(AppError::validation("Empty image data"));
        }
        Ok(Bytes::copy_from_slice(data))
    }

    /// Generate thumbnails at multiple sizes.
    pub async fn generate_multiple(
        &self,
        source_path: &str,
        file_id: uuid::Uuid,
        sizes: &[u32],
    ) -> AppResult<Vec<String>> {
        let mut paths = Vec::new();
        for &size in sizes {
            let path = self.generate(source_path, file_id, size).await?;
            paths.push(path);
        }
        Ok(paths)
    }

    /// Delete all thumbnails for a file.
    pub async fn delete_thumbnails(&self, file_id: uuid::Uuid) -> AppResult<()> {
        let dir = format!("{}/{}", self.output_dir, file_id);
        self.provider.delete_dir(&dir).await
    }
}
