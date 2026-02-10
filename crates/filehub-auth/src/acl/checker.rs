//! ACL permission checking against stored ACL entries.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::permission::AclRepository;
use filehub_entity::permission::{AclEntry, AclPermission, ResourceType};

/// Checks resource-level ACL permissions from the database.
#[derive(Debug, Clone)]
pub struct AclChecker {
    /// ACL repository.
    repo: Arc<AclRepository>,
}

impl AclChecker {
    /// Creates a new ACL checker.
    pub fn new(repo: Arc<AclRepository>) -> Self {
        Self { repo }
    }

    /// Gets the direct ACL entries for a specific resource and user.
    pub async fn get_entries_for_user(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<AclEntry>, AppError> {
        self.repo
            .find_for_user(resource_type, resource_id, user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query ACL entries: {e}")))
    }

    /// Gets all ACL entries for a resource (including public/anyone entries).
    pub async fn get_all_entries(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Vec<AclEntry>, AppError> {
        self.repo
            .find_for_resource(resource_type, resource_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query ACL entries: {e}")))
    }

    /// Checks whether a user has at least the specified permission on a resource.
    ///
    /// Permission hierarchy: Owner > Editor > Commenter > Viewer
    pub async fn check_permission(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
        required: AclPermission,
    ) -> Result<bool, AppError> {
        let entries = self
            .get_entries_for_user(resource_type, resource_id, user_id)
            .await?;

        // Check user-specific entries
        for entry in &entries {
            // Skip expired entries
            if let Some(expires) = entry.expires_at {
                if expires <= chrono::Utc::now() {
                    continue;
                }
            }

            if permission_level(&entry.permission) >= permission_level(&required) {
                return Ok(true);
            }
        }

        // Check public (is_anyone) entries
        let public_entries = self
            .repo
            .find_public_entries(resource_type, resource_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query public ACL: {e}")))?;

        for entry in &public_entries {
            if let Some(expires) = entry.expires_at {
                if expires <= chrono::Utc::now() {
                    continue;
                }
            }

            if permission_level(&entry.permission) >= permission_level(&required) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns the highest permission a user has on a resource, or None.
    pub async fn get_highest_permission(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AclPermission>, AppError> {
        let entries = self
            .get_entries_for_user(resource_type, resource_id, user_id)
            .await?;

        let public_entries = self
            .repo
            .find_public_entries(resource_type, resource_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query public ACL: {e}")))?;

        let now = chrono::Utc::now();

        let highest = entries
            .iter()
            .chain(public_entries.iter())
            .filter(|e| e.expires_at.map(|exp| exp > now).unwrap_or(true))
            .map(|e| &e.permission)
            .max_by_key(|p| permission_level(p));

        Ok(highest.cloned())
    }
}

/// Maps ACL permission levels to numeric values for comparison.
///
/// Higher means more permissive.
pub fn permission_level(perm: &AclPermission) -> u8 {
    match perm {
        AclPermission::Viewer => 0,
        AclPermission::Commenter => 1,
        AclPermission::Editor => 2,
        AclPermission::Owner => 3,
    }
}
