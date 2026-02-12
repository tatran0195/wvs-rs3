//! Core file CRUD operations with ACL permission enforcement.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_auth::rbac::RbacEnforcer;
use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::folder::FolderRepository;
use filehub_entity::file::{CreateFile, File};
use filehub_entity::permission::{AclPermission, ResourceType};

use crate::context::RequestContext;

/// Handles core file CRUD with ACL permission checks.
#[derive(Debug, Clone)]
pub struct FileService {
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Folder repository (for parent lookups).
    folder_repo: Arc<FolderRepository>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

/// Data for updating a file's metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateFileRequest {
    /// New file name.
    pub name: Option<String>,
    /// Updated metadata JSON.
    pub metadata: Option<serde_json::Value>,
}

/// Data for moving a file to a different folder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoveFileRequest {
    /// Target folder ID.
    pub target_folder_id: Uuid,
}

/// Data for copying a file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CopyFileRequest {
    /// Target folder ID.
    pub target_folder_id: Uuid,
    /// New file name (optional — defaults to original).
    pub new_name: Option<String>,
}

impl FileService {
    /// Creates a new file service.
    pub fn new(
        file_repo: Arc<FileRepository>,
        folder_repo: Arc<FolderRepository>,
        perm_resolver: Arc<EffectivePermissionResolver>,
        rbac: Arc<RbacEnforcer>,
    ) -> Self {
        Self {
            file_repo,
            folder_repo,
            perm_resolver,
            rbac,
        }
    }

    /// Lists files in a folder with pagination, enforcing viewer permission.
    pub async fn list_files(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
        page: PageRequest,
    ) -> Result<PageResponse<File>, AppError> {
        let folder = self
            .folder_repo
            .find_by_id(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Viewer,
            )
            .await?;

        self.file_repo
            .find_by_folder(folder_id, &page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list files: {e}")))
    }

    /// Gets a single file's details, enforcing viewer permission.
    pub async fn get_file(&self, ctx: &RequestContext, file_id: Uuid) -> Result<File, AppError> {
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

        Ok(file)
    }

    /// Updates a file's metadata, enforcing editor permission.
    pub async fn update_file(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        req: UpdateFileRequest,
    ) -> Result<File, AppError> {
        let mut file = self
            .get_file_with_permission(ctx, file_id, AclPermission::Editor)
            .await?;

        if let Some(name) = req.name {
            if name.trim().is_empty() {
                return Err(AppError::validation("File name cannot be empty"));
            }
            // Check for name conflict in the same folder
            if let Some(existing) = self
                .file_repo
                .find_by_folder_and_name(file.folder_id, &name)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            {
                if existing.id != file_id {
                    return Err(AppError::conflict(format!(
                        "A file named '{name}' already exists in this folder"
                    )));
                }
            }
            file.name = name;
        }

        file.metadata = req.metadata;

        file.updated_at = Utc::now();

        self.file_repo
            .update(&file)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update file: {e}")))?;

        info!(user_id = %ctx.user_id, file_id = %file_id, "File updated");

        Ok(file)
    }

    /// Moves a file to a different folder, enforcing editor permission on both source and target.
    pub async fn move_file(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        req: MoveFileRequest,
    ) -> Result<File, AppError> {
        let mut file = self
            .get_file_with_permission(ctx, file_id, AclPermission::Editor)
            .await?;

        // Check permission on target folder
        let target = self
            .folder_repo
            .find_by_id(req.target_folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                req.target_folder_id,
                target.owner_id,
                target.parent_id,
                AclPermission::Editor,
            )
            .await?;

        // Check for name conflict
        if let Some(existing) = self
            .file_repo
            .find_by_folder_and_name(req.target_folder_id, &file.name)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
        {
            if existing.id != file_id {
                return Err(AppError::conflict(format!(
                    "A file named '{}' already exists in the target folder",
                    file.name
                )));
            }
        }

        let old_folder = file.folder_id;
        file.folder_id = req.target_folder_id;
        file.storage_id = target.storage_id;
        file.updated_at = Utc::now();

        self.file_repo
            .update(&file)
            .await
            .map_err(|e| AppError::internal(format!("Failed to move file: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            file_id = %file_id,
            from_folder = %old_folder,
            to_folder = %req.target_folder_id,
            "File moved"
        );

        Ok(file)
    }

    /// Copies a file to another folder, creating a new file record.
    pub async fn copy_file(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        req: CopyFileRequest,
    ) -> Result<File, AppError> {
        let source = self
            .get_file_with_permission(ctx, file_id, AclPermission::Viewer)
            .await?;

        // Check target folder permission
        let target = self
            .folder_repo
            .find_by_id(req.target_folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                req.target_folder_id,
                target.owner_id,
                target.parent_id,
                AclPermission::Editor,
            )
            .await?;

        let new_name = req.new_name.unwrap_or_else(|| source.name.clone());

        // Check for name conflict
        if self
            .file_repo
            .find_by_folder_and_name(req.target_folder_id, &new_name)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .is_some()
        {
            return Err(AppError::conflict(format!(
                "A file named '{new_name}' already exists in the target folder"
            )));
        }

        let new_file = CreateFile {
            folder_id: req.target_folder_id,
            storage_id: target.storage_id,
            name: new_name,
            storage_path: source.storage_path.clone(),
            mime_type: source.mime_type.clone(),
            size_bytes: source.size_bytes,
            checksum_sha256: source.checksum_sha256.clone(),
            metadata: source.metadata.clone(),
            owner_id: ctx.user_id,
        };

        let new_file = self.file_repo.create(&new_file).await?;

        info!(
            user_id = %ctx.user_id,
            source_id = %file_id,
            new_id = %new_file.id,
            "File copied"
        );

        Ok(new_file)
    }

    /// Deletes a file, enforcing owner or editor permission.
    pub async fn delete_file(&self, ctx: &RequestContext, file_id: Uuid) -> Result<(), AppError> {
        let file = self
            .get_file_with_permission(ctx, file_id, AclPermission::Editor)
            .await?;

        if file.is_locked.unwrap_or(false) && file.locked_by != Some(ctx.user_id) && !ctx.is_admin()
        {
            return Err(AppError::conflict("File is locked by another user"));
        }

        self.file_repo
            .delete(file_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete file: {e}")))?;

        info!(user_id = %ctx.user_id, file_id = %file_id, "File deleted");

        Ok(())
    }

    /// Locks a file for exclusive editing.
    pub async fn lock_file(&self, ctx: &RequestContext, file_id: Uuid) -> Result<File, AppError> {
        let mut file = self
            .get_file_with_permission(ctx, file_id, AclPermission::Editor)
            .await?;

        if file.is_locked.unwrap_or(false) {
            if file.locked_by == Some(ctx.user_id) {
                return Ok(file); // Already locked by this user
            }
            return Err(AppError::conflict("File is already locked by another user"));
        }

        file.is_locked = Some(true);
        file.locked_by = Some(ctx.user_id);
        file.locked_at = Some(Utc::now());
        file.updated_at = Utc::now();

        self.file_repo
            .update(&file)
            .await
            .map_err(|e| AppError::internal(format!("Failed to lock file: {e}")))?;

        info!(user_id = %ctx.user_id, file_id = %file_id, "File locked");

        Ok(file)
    }

    /// Unlocks a file (owner of lock or admin).
    pub async fn unlock_file(&self, ctx: &RequestContext, file_id: Uuid) -> Result<File, AppError> {
        let mut file = self.get_file(ctx, file_id).await?;

        if !file.is_locked.unwrap_or(false) {
            return Ok(file);
        }

        if file.locked_by != Some(ctx.user_id) && !ctx.is_admin() {
            return Err(AppError::forbidden(
                "Only the lock owner or an admin can unlock this file",
            ));
        }

        file.is_locked = Some(false);
        file.locked_by = None;
        file.locked_at = None;
        file.updated_at = Utc::now();

        self.file_repo
            .update(&file)
            .await
            .map_err(|e| AppError::internal(format!("Failed to unlock file: {e}")))?;

        info!(user_id = %ctx.user_id, file_id = %file_id, "File unlocked");

        Ok(file)
    }

    /// Internal helper — loads a file and checks the required ACL permission.
    async fn get_file_with_permission(
        &self,
        ctx: &RequestContext,
        file_id: Uuid,
        required: AclPermission,
    ) -> Result<File, AppError> {
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
                required,
            )
            .await?;

        Ok(file)
    }
}
