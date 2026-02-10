//! Preview and thumbnail generation service.

use std::sync::Arc;

use bytes::Bytes;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_cache::provider::CacheManager;
use filehub_core::error::AppError;
use filehub_database::repositories::file::FileRepository;
use filehub_entity::permission::{AclPermission, ResourceType};
use filehub_storage::manager::StorageManager;

use crate::context::RequestContext;

/// Generates and serves file previews/thumbnails.
#[derive(Clone)]
pub struct PreviewService {
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Storage manager.
    storage: Arc<StorageManager>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
    /// Cache for storing generated thumbnails.
    cache: Arc<CacheManager>,
}

impl std::fmt::Debug for PreviewService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewService").finish()
    }
}

/// Result of a preview request.
#[derive(Debug)]
pub struct PreviewResult {
    /// Preview image data.
    pub data: Bytes,
    /// MIME type.
    pub content_type: String,
}

impl PreviewService {
    /// Creates a new preview service.
    pub fn new(
        file_repo: Arc<FileRepository>,
        storage: Arc<StorageManager>,
        perm_resolver: Arc<EffectivePermissionResolver>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            file_repo,
            storage,
            perm_resolver,
            cache,
        }
    }

    /// Gets or generates a preview/thumbnail for a file.
    pub async fn get_preview(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        size: Option<u32>,
    ) -> Result<PreviewResult, AppError> {
        let file = self
            .file_repo
            .find_by_id(file_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("File not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::File,
                file_id,
                file.owner_id,
                Some(file.folder_id),
                AclPermission::Viewer,
            )
            .await?;

        let thumb_size = size.unwrap_or(256);
        let cache_key = format!("preview:{}:{}", file_id, thumb_size);

        // Check cache
        if let Ok(Some(cached)) = self.cache.get_bytes(&cache_key).await {
            return Ok(PreviewResult {
                data: Bytes::from(cached),
                content_type: "image/png".to_string(),
            });
        }

        // Check if file is an image
        let mime = file
            .mime_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        if !mime.starts_with("image/") {
            return Err(AppError::validation(
                "Preview is only available for image files",
            ));
        }

        // Read original file
        let original = self
            .storage
            .read(file.storage_id, &file.storage_path)
            .await
            .map_err(|e| AppError::internal(format!("Storage read failed: {e}")))?;

        // Generate thumbnail (basic resize)
        let thumbnail = self.generate_thumbnail(&original, thumb_size)?;

        // Cache thumbnail for 1 hour
        let _ = self
            .cache
            .set_bytes_with_ttl(&cache_key, &thumbnail, std::time::Duration::from_secs(3600))
            .await;

        Ok(PreviewResult {
            data: Bytes::from(thumbnail),
            content_type: "image/png".to_string(),
        })
    }

    /// Generates a thumbnail from raw image bytes.
    fn generate_thumbnail(&self, data: &[u8], max_size: u32) -> Result<Vec<u8>, AppError> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::internal(format!("Failed to decode image: {e}")))?;

        let thumb = img.thumbnail(max_size, max_size);

        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        thumb
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| AppError::internal(format!("Failed to encode thumbnail: {e}")))?;

        Ok(buf)
    }
}
