//! ACL repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_entity::permission::acl::AclPermission;
use filehub_entity::permission::model::{AclEntry, ResourceType};

/// Repository for ACL entry CRUD and permission resolution.
#[derive(Debug, Clone)]
pub struct AclRepository {
    pool: PgPool,
}

impl AclRepository {
    /// Create a new ACL repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find an ACL entry by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<AclEntry>> {
        sqlx::query_as::<_, AclEntry>("SELECT * FROM acl_entries WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find ACL entry", e))
    }

    /// Find all ACL entries for a resource.
    pub async fn find_by_resource(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<Vec<AclEntry>> {
        sqlx::query_as::<_, AclEntry>(
            "SELECT * FROM acl_entries WHERE resource_type = $1 AND resource_id = $2 \
             AND (expires_at IS NULL OR expires_at > NOW()) ORDER BY created_at ASC",
        )
        .bind(&resource_type)
        .bind(resource_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find ACL entries", e))
    }

    /// Find a user's permission on a specific resource.
    pub async fn find_user_permission(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<AclEntry>> {
        sqlx::query_as::<_, AclEntry>(
            "SELECT * FROM acl_entries \
             WHERE resource_type = $1 AND resource_id = $2 AND (user_id = $3 OR is_anyone = TRUE) \
             AND (expires_at IS NULL OR expires_at > NOW()) \
             ORDER BY CASE WHEN user_id = $3 THEN 0 ELSE 1 END, permission ASC \
             LIMIT 1",
        )
        .bind(&resource_type)
        .bind(resource_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find user permission", e)
        })
    }

    /// Resolve effective permission for a user on a folder by walking up the tree.
    pub async fn resolve_effective_permission(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<AclPermission>> {
        // Walk up the folder hierarchy looking for an ACL entry
        let rows = sqlx::query_as::<_, AclEntry>(
            "WITH RECURSIVE ancestors AS ( \
                SELECT id, parent_id, 0 as level FROM folders WHERE id = $1 \
                UNION ALL \
                SELECT f.id, f.parent_id, a.level + 1 FROM folders f \
                INNER JOIN ancestors a ON f.id = a.parent_id \
             ) \
             SELECT ae.* FROM acl_entries ae \
             INNER JOIN ancestors anc ON ae.resource_id = anc.id \
             WHERE ae.resource_type = 'folder' \
             AND (ae.user_id = $2 OR ae.is_anyone = TRUE) \
             AND (ae.expires_at IS NULL OR ae.expires_at > NOW()) \
             ORDER BY anc.level ASC",
        )
        .bind(folder_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to resolve permissions", e)
        })?;

        // Find the first applicable entry (closest in the tree)
        // Respect inheritance blocking
        for entry in &rows {
            if entry.inheritance == filehub_entity::permission::acl::AclInheritance::Block {
                // If blocked, only use this entry if it directly matches
                if entry.resource_id == folder_id {
                    return Ok(Some(entry.permission));
                }
                // Otherwise, stop looking further up
                return Ok(None);
            }
            return Ok(Some(entry.permission));
        }

        Ok(None)
    }

    /// Create a new ACL entry.
    pub async fn create(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Option<Uuid>,
        is_anyone: bool,
        permission: AclPermission,
        inheritance: filehub_entity::permission::acl::AclInheritance,
        granted_by: Uuid,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<AclEntry> {
        sqlx::query_as::<_, AclEntry>(
            "INSERT INTO acl_entries (resource_type, resource_id, user_id, is_anyone, permission, inheritance, granted_by, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
            .bind(&resource_type)
            .bind(resource_id)
            .bind(user_id)
            .bind(is_anyone)
            .bind(&permission)
            .bind(&inheritance)
            .bind(granted_by)
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create ACL entry", e))
    }

    /// Update an ACL entry's permission.
    pub async fn update_permission(
        &self,
        entry_id: Uuid,
        permission: AclPermission,
    ) -> AppResult<AclEntry> {
        sqlx::query_as::<_, AclEntry>(
            "UPDATE acl_entries SET permission = $2 WHERE id = $1 RETURNING *",
        )
        .bind(entry_id)
        .bind(&permission)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update ACL entry", e))?
        .ok_or_else(|| AppError::not_found(format!("ACL entry {entry_id} not found")))
    }

    /// Delete an ACL entry.
    pub async fn delete(&self, entry_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM acl_entries WHERE id = $1")
            .bind(entry_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to delete ACL entry", e)
            })?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete all ACL entries for a resource.
    pub async fn delete_by_resource(
        &self,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<u64> {
        let result =
            sqlx::query("DELETE FROM acl_entries WHERE resource_type = $1 AND resource_id = $2")
                .bind(&resource_type)
                .bind(resource_id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to delete ACL entries", e)
                })?;
        Ok(result.rows_affected())
    }
}
