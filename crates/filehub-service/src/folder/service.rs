//! Folder CRUD operations with ACL enforcement.

use std::sync::Arc;

use chrono::Utc;
use filehub_core::types::{PageRequest, PageResponse};
use tracing::info;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_core::error::AppError;
use filehub_database::repositories::folder::FolderRepository;
use filehub_database::repositories::storage::StorageRepository;
use filehub_entity::folder::{CreateFolder, Folder};
use filehub_entity::permission::{AclPermission, ResourceType};

use crate::context::RequestContext;

/// Manages folder CRUD operations.
#[derive(Debug, Clone)]
pub struct FolderService {
    /// Folder repository.
    folder_repo: Arc<FolderRepository>,
    /// Storage repository.
    storage_repo: Arc<StorageRepository>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
}

/// Request to create a new folder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateFolderRequest {
    /// Storage ID.
    pub storage_id: Uuid,
    /// Parent folder ID (None for root-level).
    pub parent_id: Option<Uuid>,
    /// Folder name.
    pub name: String,
}

/// Request to move a folder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoveFolderRequest {
    /// New parent folder ID.
    pub new_parent_id: Uuid,
}

impl FolderService {
    /// Creates a new folder service.
    pub fn new(
        folder_repo: Arc<FolderRepository>,
        storage_repo: Arc<StorageRepository>,
        perm_resolver: Arc<EffectivePermissionResolver>,
    ) -> Self {
        Self {
            folder_repo,
            storage_repo,
            perm_resolver,
        }
    }

    /// Lists root folders for a storage.
    pub async fn list_root_folders(
        &self,
        _ctx: &RequestContext,
        storage_id: Uuid,
    ) -> Result<Vec<Folder>, AppError> {
        self.folder_repo
            .find_roots(storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list folders: {e}")))
    }

    /// Gets a folder by ID.
    pub async fn get_folder(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
    ) -> Result<Folder, AppError> {
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

        Ok(folder)
    }

    /// Lists children of a folder.
    pub async fn list_children(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
        page: PageRequest,
    ) -> Result<PageResponse<Folder>, AppError> {
        self.folder_repo
            .find_children(folder_id, &page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list children: {e}")))
    }

    /// Creates a new folder.
    pub async fn create_folder(
        &self,
        ctx: &RequestContext,
        req: CreateFolderRequest,
    ) -> Result<Folder, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::validation("Folder name cannot be empty"));
        }

        // Verify storage exists
        self.storage_repo
            .find_by_id(req.storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Storage not found"))?;

        // Determine path and depth
        let (path, depth) = if let Some(parent_id) = req.parent_id {
            let parent = self.get_folder(ctx, parent_id).await?;

            // Check editor permission on parent
            self.perm_resolver
                .require_permission(
                    ctx.user_id,
                    &ctx.role,
                    ResourceType::Folder,
                    parent_id,
                    parent.owner_id,
                    parent.parent_id,
                    AclPermission::Editor,
                )
                .await?;

            let path = format!("{}/{}", parent.path, req.name);
            (path, parent.depth + 1)
        } else {
            (format!("/{}", req.name), 0)
        };

        // Check for name conflict
        if self
            .folder_repo
            .find_by_path(req.storage_id, &path)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .is_some()
        {
            return Err(AppError::conflict(format!(
                "A folder at path '{}' already exists",
                path
            )));
        }

        let folder_record = CreateFolder {
            storage_id: req.storage_id,
            parent_id: req.parent_id,
            name: req.name,
            path,
            depth,
            owner_id: ctx.user_id,
        };

        let folder = self
            .folder_repo
            .create(&folder_record)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create folder: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            folder_id = %folder.id,
            path = %folder.path,
            "Folder created"
        );

        Ok(folder)
    }

    /// Renames / updates a folder.
    pub async fn update_folder(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
        new_name: &str,
    ) -> Result<Folder, AppError> {
        if new_name.trim().is_empty() {
            return Err(AppError::validation("Folder name cannot be empty"));
        }

        let mut folder = self.get_folder(ctx, folder_id).await?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Editor,
            )
            .await?;

        // Update path
        let old_path = folder.path.clone();
        folder.name = new_name.to_string();

        if let Some(last_slash) = folder.path.rfind('/') {
            folder.path = format!("{}/{}", &folder.path[..last_slash], new_name);
        } else {
            folder.path = format!("/{}", new_name);
        }

        folder.updated_at = Utc::now();

        self.folder_repo
            .update(&folder)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update folder: {e}")))?;

        // Update child paths
        self.folder_repo
            .update_children_paths(&old_path, &folder.path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update child paths: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            folder_id = %folder_id,
            new_name = %new_name,
            "Folder renamed"
        );

        Ok(folder)
    }

    /// Moves a folder to a new parent.
    pub async fn move_folder(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
        req: MoveFolderRequest,
    ) -> Result<Folder, AppError> {
        let mut folder = self.get_folder(ctx, folder_id).await?;

        // Cannot move to self
        if folder_id == req.new_parent_id {
            return Err(AppError::validation("Cannot move a folder into itself"));
        }

        // Check permissions on source
        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Editor,
            )
            .await?;

        // Check target exists and permissions
        let target = self
            .folder_repo
            .find_by_id(req.new_parent_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                req.new_parent_id,
                target.owner_id,
                target.parent_id,
                AclPermission::Editor,
            )
            .await?;

        // Check for circular reference
        let target_ancestors = self
            .folder_repo
            .get_ancestry(req.new_parent_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?;

        if target_ancestors.contains(&folder_id) {
            return Err(AppError::validation(
                "Cannot move a folder into one of its descendants",
            ));
        }

        let old_path = folder.path.clone();
        folder.parent_id = Some(req.new_parent_id);
        folder.path = format!("{}/{}", target.path, folder.name);
        folder.depth = target.depth + 1;
        folder.updated_at = Utc::now();

        self.folder_repo
            .update(&folder)
            .await
            .map_err(|e| AppError::internal(format!("Failed to move folder: {e}")))?;

        // Update child paths
        self.folder_repo
            .update_children_paths(&old_path, &folder.path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update child paths: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            folder_id = %folder_id,
            new_parent = %req.new_parent_id,
            "Folder moved"
        );

        Ok(folder)
    }

    /// Deletes a folder and all its contents.
    pub async fn delete_folder(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
    ) -> Result<(), AppError> {
        let folder = self.get_folder(ctx, folder_id).await?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Owner,
            )
            .await?;

        self.folder_repo
            .delete(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete folder: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            folder_id = %folder_id,
            path = %folder.path,
            "Folder deleted"
        );

        Ok(())
    }
}
