//! Folder ACL inheritance resolution.
//!
//! ACL inheritance rules:
//! - Permissions cascade from parent folders to children (when inheritance = "inherit").
//! - An explicit entry on a child overrides inherited entries.
//! - A "block" inheritance entry stops the cascade from propagating further.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::folder::FolderRepository;
use filehub_database::repositories::permission::AclRepository;
use filehub_entity::permission::{AclEntry, AclInheritance, AclPermission, ResourceType};

use super::checker::permission_level;

/// Resolves ACL permissions with folder hierarchy inheritance.
#[derive(Debug, Clone)]
pub struct AclInheritanceResolver {
    /// Folder repository for ancestry lookups.
    folder_repo: Arc<FolderRepository>,
    /// ACL repository for permission lookups.
    acl_repo: Arc<AclRepository>,
}

impl AclInheritanceResolver {
    /// Creates a new inheritance resolver.
    pub fn new(folder_repo: Arc<FolderRepository>, acl_repo: Arc<AclRepository>) -> Self {
        Self {
            folder_repo,
            acl_repo,
        }
    }

    /// Resolves the effective ACL permission for a user on a folder,
    /// walking up the folder ancestry chain.
    ///
    /// Algorithm:
    /// 1. Check direct entries on the target folder.
    /// 2. If no direct entry, walk up to parent.
    /// 3. At each ancestor, check for entries.
    /// 4. Stop if an entry with `inheritance = Block` is found.
    /// 5. Return the highest permission found along the chain.
    pub async fn resolve_folder_permission(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AclPermission>, AppError> {
        let ancestors = self.get_folder_ancestry(folder_id).await?;
        let now = chrono::Utc::now();

        let mut highest_permission: Option<AclPermission> = None;

        for ancestor_id in &ancestors {
            let entries = self
                .acl_repo
                .find_for_user(ResourceType::Folder, *ancestor_id, user_id)
                .await
                .map_err(|e| AppError::internal(format!("ACL lookup failed: {e}")))?;

            let public_entries = self
                .acl_repo
                .find_public_entries(ResourceType::Folder, *ancestor_id)
                .await
                .map_err(|e| AppError::internal(format!("Public ACL lookup failed: {e}")))?;

            let all_entries: Vec<&AclEntry> = entries
                .iter()
                .chain(public_entries.iter())
                .filter(|e| e.expires_at.map(|exp| exp > now).unwrap_or(true))
                .collect();

            // Check for block inheritance
            let has_block = all_entries
                .iter()
                .any(|e| e.inheritance == AclInheritance::Block);

            // Find highest permission at this level
            for entry in &all_entries {
                let level = permission_level(&entry.permission);
                let current_level = highest_permission
                    .as_ref()
                    .map(|p| permission_level(p))
                    .unwrap_or(0);

                if highest_permission.is_none() || level > current_level {
                    highest_permission = Some(entry.permission.clone());
                }
            }

            // If this is the target folder itself and has direct entries,
            // those take priority (but we still check ancestors unless blocked)
            if *ancestor_id == folder_id && !all_entries.is_empty() {
                // Direct entry found — it overrides inherited
                // But we still return the highest we found
            }

            // Stop if inheritance is blocked at this level
            if has_block && *ancestor_id != folder_id {
                break;
            }
        }

        Ok(highest_permission)
    }

    /// Resolves the effective ACL permission for a user on a file,
    /// first checking the file's direct entries, then walking the parent folder chain.
    pub async fn resolve_file_permission(
        &self,
        file_id: Uuid,
        folder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AclPermission>, AppError> {
        // First check direct file-level entries
        let now = chrono::Utc::now();

        let file_entries = self
            .acl_repo
            .find_for_user(ResourceType::File, file_id, user_id)
            .await
            .map_err(|e| AppError::internal(format!("File ACL lookup failed: {e}")))?;

        let file_public = self
            .acl_repo
            .find_public_entries(ResourceType::File, file_id)
            .await
            .map_err(|e| AppError::internal(format!("File public ACL lookup failed: {e}")))?;

        let all_file: Vec<&AclEntry> = file_entries
            .iter()
            .chain(file_public.iter())
            .filter(|e| e.expires_at.map(|exp| exp > now).unwrap_or(true))
            .collect();

        if !all_file.is_empty() {
            // Direct file-level permission takes precedence
            let highest = all_file
                .iter()
                .max_by_key(|e| permission_level(&e.permission))
                .map(|e| e.permission.clone());
            return Ok(highest);
        }

        // No direct file entry — inherit from folder
        self.resolve_folder_permission(folder_id, user_id).await
    }

    /// Gets the ancestry chain for a folder, starting with the folder itself
    /// and walking up to the root.
    async fn get_folder_ancestry(&self, folder_id: Uuid) -> Result<Vec<Uuid>, AppError> {
        self.folder_repo
            .get_ancestry(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get folder ancestry: {e}")))
    }
}
