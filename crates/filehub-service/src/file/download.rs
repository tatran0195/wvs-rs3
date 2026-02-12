//! File download service — streams file content with ACL enforcement.

use std::sync::Arc;

use bytes::Bytes;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_core::error::AppError;
use filehub_database::repositories::file::FileRepository;
use filehub_entity::file::File;
use filehub_entity::permission::{AclPermission, ResourceType};
use filehub_storage::manager::StorageManager;

use crate::context::RequestContext;

/// Handles file downloads with ACL checking and streaming.
#[derive(Clone)]
pub struct DownloadService {
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Storage manager.
    storage: Arc<StorageManager>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
}

impl std::fmt::Debug for DownloadService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadService").finish()
    }
}

/// Result containing file metadata and content bytes for a download.
#[derive(Debug)]
pub struct DownloadResult {
    /// File metadata.
    pub file: File,
    /// File content bytes.
    pub data: Bytes,
    /// MIME type for Content-Type header.
    pub content_type: String,
    /// Suggested filename for Content-Disposition.
    pub filename: String,
}

impl DownloadService {
    /// Creates a new download service.
    pub fn new(
        file_repo: Arc<FileRepository>,
        storage: Arc<StorageManager>,
        perm_resolver: Arc<EffectivePermissionResolver>,
    ) -> Self {
        Self {
            file_repo,
            storage,
            perm_resolver,
        }
    }

    /// Downloads a file, checking viewer permission.
    pub async fn download(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
    ) -> Result<DownloadResult, AppError> {
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

        let data = self
            .storage
            .read(&file.storage_id, &file.storage_path)
            .await
            .map_err(|e| AppError::internal(format!("Storage read failed: {e}")))?;

        let content_type = file
            .mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        Ok(DownloadResult {
            filename: file.name.clone(),
            file,
            data,
            content_type,
        })
    }

    /// Downloads a specific version of a file.
    pub async fn download_version(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        version_number: i32,
    ) -> Result<DownloadResult, AppError> {
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

        let version = self
            .file_repo
            .find_version(file_id, version_number)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("Version {version_number} not found")))?;

        let provider = self
            .storage
            .get(&file.storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Storage provider not found: {e}")))?;

        let data = provider
            .read_bytes(&version.storage_path)
            .await
            .map_err(|e| AppError::internal(format!("Storage read failed: {e}")))?;

        let content_type = file
            .mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        Ok(DownloadResult {
            filename: file.name.clone(),
            file,
            data,
            content_type,
        })
    }

    /// Downloads a file via a share token (no auth context required).
    pub async fn download_via_share(
        &self,
        file_id: Uuid,
        storage_id: Uuid,
        storage_path: &str,
        mime_type: Option<&str>,
        filename: &str,
    ) -> Result<DownloadResult, AppError> {
        let file = self
            .file_repo
            .find_by_id(file_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("File not found"))?;

        let provider = self
            .storage
            .get(&storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Storage provider not found: {e}")))?;

        let data = provider
            .read_bytes(storage_path)
            .await
            .map_err(|e| AppError::internal(format!("Storage read failed: {e}")))?;

        let content_type = mime_type.unwrap_or("application/octet-stream").to_string();

        Ok(DownloadResult {
            filename: filename.to_string(),
            file,
            data,
            content_type,
        })
    }
}
