//! File versioning service — create, list, and restore versions.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_core::error::AppError;
use filehub_database::repositories::file::FileRepository;
use filehub_entity::file::{File, FileVersion};
use filehub_entity::permission::{AclPermission, ResourceType};

use crate::context::RequestContext;

/// Manages file version history.
#[derive(Debug, Clone)]
pub struct VersionService {
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
}

impl VersionService {
    /// Creates a new version service.
    pub fn new(
        file_repo: Arc<FileRepository>,
        perm_resolver: Arc<EffectivePermissionResolver>,
    ) -> Self {
        Self {
            file_repo,
            perm_resolver,
        }
    }

    /// Lists all versions of a file.
    pub async fn list_versions(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
    ) -> Result<Vec<FileVersion>, AppError> {
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

        self.file_repo
            .find_versions(file_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list versions: {e}")))
    }

    /// Creates a new version snapshot of the current file state.
    pub async fn create_version(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        comment: Option<&str>,
    ) -> Result<FileVersion, AppError> {
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
                AclPermission::Editor,
            )
            .await?;

        let version = FileVersion {
            id: Uuid::new_v4(),
            file_id,
            version_number: file.current_version,
            storage_path: file.storage_path.clone(),
            size_bytes: file.size_bytes,
            checksum_sha256: file.checksum_sha256.clone(),
            created_by: ctx.user_id,
            created_at: Utc::now(),
            comment: comment.map(String::from),
        };

        self.file_repo
            .create_version(&version)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create version: {e}")))?;

        // Increment current version
        self.file_repo
            .increment_version(file_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to increment version: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            file_id = %file_id,
            version = version.version_number,
            "File version created"
        );

        Ok(version)
    }
}
