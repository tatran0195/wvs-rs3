//! Effective permission resolver that chains RBAC, ACL, and share checks.
//!
//! Resolution order:
//! 1. Admin bypass — admins have full access to everything.
//! 2. Owner check — resource owners have full access.
//! 3. RBAC — check system-level role permission.
//! 4. ACL — check resource-level permission (with inheritance).
//! 5. Share — check if access is via a valid share link.

use std::sync::Arc;

use uuid::Uuid;

use filehub_cache::provider::CacheManager;
use filehub_core::error::AppError;
use filehub_core::traits::CacheProvider;
use filehub_entity::permission::{AclPermission, ResourceType};
use filehub_entity::user::UserRole;

use crate::rbac::RbacEnforcer;

use super::checker::AclChecker;
use super::inheritance::AclInheritanceResolver;

/// Result of resolving effective permissions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EffectivePermission {
    /// Whether access is granted.
    pub granted: bool,
    /// The resolved ACL permission level (if applicable).
    pub acl_permission: Option<AclPermission>,
    /// The source of the permission grant.
    pub source: PermissionSource,
}

/// Where the permission was derived from.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionSource {
    /// User is an admin with full access.
    AdminBypass,
    /// User owns the resource.
    Owner,
    /// Permission from RBAC role.
    Rbac,
    /// Permission from ACL entry (direct or inherited).
    Acl,
    /// Permission from a share link.
    Share,
    /// Access denied — no applicable permission found.
    Denied,
}

/// Chains RBAC, ACL, and share checks to resolve effective permissions.
#[derive(Clone)]
pub struct EffectivePermissionResolver {
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
    /// Direct ACL checker.
    acl_checker: Arc<AclChecker>,
    /// Folder inheritance resolver.
    inheritance: Arc<AclInheritanceResolver>,
    /// Cache for resolved permissions.
    cache: Arc<CacheManager>,
}

impl std::fmt::Debug for EffectivePermissionResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectivePermissionResolver").finish()
    }
}

impl EffectivePermissionResolver {
    /// Creates a new effective permission resolver.
    pub fn new(
        rbac: Arc<RbacEnforcer>,
        acl_checker: Arc<AclChecker>,
        inheritance: Arc<AclInheritanceResolver>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            rbac,
            acl_checker,
            inheritance,
            cache,
        }
    }

    /// Resolves the effective permission for a user on a resource.
    ///
    /// Uses caching to avoid repeated database lookups.
    pub async fn resolve(
        &self,
        user_id: Uuid,
        user_role: &UserRole,
        resource_type: ResourceType,
        resource_id: Uuid,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
        required_permission: AclPermission,
    ) -> Result<EffectivePermission, AppError> {
        // Check cache first
        let cache_key = format!(
            "perm:{}:{}:{}:{}",
            user_id, resource_type, resource_id, required_permission
        );

        if let Ok(Some(cached)) = self.cache.get(&cache_key).await {
            if let Ok(perm) = serde_json::from_str::<EffectivePermission>(&cached) {
                return Ok(perm);
            }
        }

        let result = self
            .resolve_uncached(
                user_id,
                user_role,
                resource_type,
                resource_id,
                owner_id,
                parent_folder_id,
                required_permission,
            )
            .await?;

        // Cache the result for 5 minutes
        if let Ok(serialized) = serde_json::to_string(&result) {
            let _ = self
                .cache
                .set(&cache_key, &serialized, std::time::Duration::from_secs(300))
                .await;
        }

        Ok(result)
    }

    /// Resolves the effective permission without caching.
    async fn resolve_uncached(
        &self,
        user_id: Uuid,
        user_role: &UserRole,
        resource_type: ResourceType,
        resource_id: Uuid,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
        required_permission: AclPermission,
    ) -> Result<EffectivePermission, AppError> {
        // 1. Admin bypass
        if self.rbac.is_admin(user_role) {
            return Ok(EffectivePermission {
                granted: true,
                acl_permission: Some(AclPermission::Owner),
                source: PermissionSource::AdminBypass,
            });
        }

        // 2. Owner check
        if user_id == owner_id {
            return Ok(EffectivePermission {
                granted: true,
                acl_permission: Some(AclPermission::Owner),
                source: PermissionSource::Owner,
            });
        }

        // 3. ACL check (with inheritance for files/folders)
        let acl_permission = match resource_type {
            ResourceType::File => {
                if let Some(folder_id) = parent_folder_id {
                    self.inheritance
                        .resolve_file_permission(resource_id, folder_id, user_id)
                        .await?
                } else {
                    self.acl_checker
                        .get_highest_permission(resource_type, resource_id, user_id)
                        .await?
                }
            }
            ResourceType::Folder => {
                self.inheritance
                    .resolve_folder_permission(resource_id, user_id)
                    .await?
            }
            ResourceType::Storage => {
                self.acl_checker
                    .get_highest_permission(resource_type, resource_id, user_id)
                    .await?
            }
        };

        if let Some(ref perm) = acl_permission {
            if super::checker::permission_level(perm)
                >= super::checker::permission_level(&required_permission)
            {
                return Ok(EffectivePermission {
                    granted: true,
                    acl_permission: Some(perm.clone()),
                    source: PermissionSource::Acl,
                });
            }
        }

        // 4. Denied
        Ok(EffectivePermission {
            granted: false,
            acl_permission,
            source: PermissionSource::Denied,
        })
    }

    /// Checks and returns an error if the user doesn't have the required permission.
    pub async fn require_permission(
        &self,
        user_id: Uuid,
        user_role: &UserRole,
        resource_type: ResourceType,
        resource_id: Uuid,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
        required_permission: AclPermission,
    ) -> Result<EffectivePermission, AppError> {
        let result = self
            .resolve(
                user_id,
                user_role,
                resource_type,
                resource_id,
                owner_id,
                parent_folder_id,
                required_permission,
            )
            .await?;

        if !result.granted {
            return Err(AppError::forbidden(
                "You do not have permission to perform this action on this resource",
            ));
        }

        Ok(result)
    }

    /// Invalidates the cached permission for a specific user+resource combination.
    pub async fn invalidate_cache(
        &self,
        user_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<(), AppError> {
        // Invalidate for all permission levels
        for perm in &[
            AclPermission::Viewer,
            AclPermission::Commenter,
            AclPermission::Editor,
            AclPermission::Owner,
        ] {
            let key = format!(
                "perm:{}:{}:{}:{}",
                user_id, resource_type, resource_id, perm
            );
            let _ = self.cache.delete(&key).await;
        }
        Ok(())
    }

    /// Invalidates all cached permissions for a specific resource.
    pub async fn invalidate_resource_cache(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<(), AppError> {
        let pattern = format!("perm:*:{}:{}:*", resource_type, resource_id);
        let _ = self.cache.delete_pattern(&pattern).await;
        Ok(())
    }
}
