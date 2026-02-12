//! ACL permission management — add, update, remove ACL entries.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_core::error::AppError;
use filehub_database::repositories::permission::AclRepository;
use filehub_entity::permission::{AclEntry, AclInheritance, AclPermission, ResourceType};

use crate::context::RequestContext;

/// Manages ACL entries on resources.
#[derive(Debug, Clone)]
pub struct PermissionService {
    /// ACL repository.
    acl_repo: Arc<AclRepository>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
    /// Permission resolver (for cache invalidation).
    perm_resolver: Arc<EffectivePermissionResolver>,
}

/// Request to create an ACL entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAclEntryRequest {
    /// User ID to grant permission to (None for public).
    pub user_id: Option<Uuid>,
    /// Whether this is a public (anyone) entry.
    pub is_anyone: bool,
    /// Permission level.
    pub permission: AclPermission,
    /// Inheritance behavior.
    pub inheritance: AclInheritance,
    /// Expiration time.
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

/// Request to update an ACL entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateAclEntryRequest {
    /// New permission level.
    pub permission: Option<AclPermission>,
    /// New inheritance behavior.
    pub inheritance: Option<AclInheritance>,
    /// New expiration.
    pub expires_at: Option<Option<chrono::DateTime<Utc>>>,
}

impl PermissionService {
    /// Creates a new permission service.
    pub fn new(
        acl_repo: Arc<AclRepository>,
        rbac: Arc<RbacEnforcer>,
        perm_resolver: Arc<EffectivePermissionResolver>,
    ) -> Self {
        Self {
            acl_repo,
            rbac,
            perm_resolver,
        }
    }

    /// Gets all ACL entries for a resource.
    pub async fn get_entries(
        &self,
        _ctx: &RequestContext,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Vec<AclEntry>, AppError> {
        self.acl_repo
            .find_for_resource(resource_type, resource_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get ACL entries: {e}")))
    }

    /// Adds a new ACL entry.
    pub async fn add_entry(
        &self,
        ctx: &RequestContext,
        resource_type: ResourceType,
        resource_id: Uuid,
        req: CreateAclEntryRequest,
    ) -> Result<AclEntry, AppError> {
        // Admins can always manage permissions; others need PermissionManageAll or ownership
        if !ctx.is_admin() {
            self.rbac
                .require_permission(&ctx.role, &SystemPermission::PermissionManageAll)?;
        }

        let entry = self
            .acl_repo
            .create(
                resource_type,
                resource_id,
                req.user_id,
                req.is_anyone,
                req.permission,
                req.inheritance,
                ctx.user_id,
                req.expires_at,
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to create ACL entry: {e}")))?;

        // Invalidate permission cache
        let _ = self
            .perm_resolver
            .invalidate_resource_cache(resource_type, resource_id)
            .await;

        info!(
            admin_id = %ctx.user_id,
            entry_id = %entry.id,
            resource = ?entry.resource_type,
            "ACL entry added"
        );

        Ok(entry)
    }

    /// Updates an existing ACL entry.
    pub async fn update_entry(
        &self,
        ctx: &RequestContext,
        entry_id: Uuid,
        req: UpdateAclEntryRequest,
    ) -> Result<AclEntry, AppError> {
        if !ctx.is_admin() {
            self.rbac
                .require_permission(&ctx.role, &SystemPermission::PermissionManageAll)?;
        }

        let mut entry = self
            .acl_repo
            .find_by_id(entry_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("ACL entry not found"))?;

        if let Some(permission) = req.permission {
            entry.permission = permission;
        }
        if let Some(inheritance) = req.inheritance {
            entry.inheritance = inheritance;
        }
        if let Some(expires_at) = req.expires_at {
            entry.expires_at = expires_at;
        }

        self.acl_repo
            .update(&entry)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update ACL entry: {e}")))?;

        let _ = self
            .perm_resolver
            .invalidate_resource_cache(entry.resource_type.clone(), entry.resource_id)
            .await;

        info!(
            admin_id = %ctx.user_id,
            entry_id = %entry_id,
            "ACL entry updated"
        );

        Ok(entry)
    }

    /// Removes an ACL entry.
    pub async fn remove_entry(&self, ctx: &RequestContext, entry_id: Uuid) -> Result<(), AppError> {
        if !ctx.is_admin() {
            self.rbac
                .require_permission(&ctx.role, &SystemPermission::PermissionManageAll)?;
        }

        let entry = self
            .acl_repo
            .find_by_id(entry_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("ACL entry not found"))?;

        self.acl_repo
            .delete(entry_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to remove ACL entry: {e}")))?;

        let _ = self
            .perm_resolver
            .invalidate_resource_cache(entry.resource_type, entry.resource_id)
            .await;

        info!(
            admin_id = %ctx.user_id,
            entry_id = %entry_id,
            "ACL entry removed"
        );

        Ok(())
    }
}
