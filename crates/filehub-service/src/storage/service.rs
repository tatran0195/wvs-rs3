//! Storage backend management.

use std::sync::Arc;

use uuid::Uuid;

use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_core::error::AppError;
use filehub_database::repositories::storage::StorageRepository;
use filehub_entity::storage::Storage;

use crate::context::RequestContext;

/// Manages storage backend CRUD and usage reporting.
#[derive(Debug, Clone)]
pub struct StorageService {
    /// Storage repository.
    storage_repo: Arc<StorageRepository>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

/// Storage usage statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageUsage {
    /// Storage ID.
    pub storage_id: Uuid,
    /// Storage name.
    pub name: String,
    /// Total used bytes.
    pub used_bytes: i64,
    /// Quota in bytes (None = unlimited).
    pub quota_bytes: Option<i64>,
    /// Usage percentage (None if no quota).
    pub usage_percent: Option<f64>,
    /// Number of files.
    pub file_count: i64,
    /// Number of folders.
    pub folder_count: i64,
}

impl StorageService {
    /// Creates a new storage service.
    pub fn new(storage_repo: Arc<StorageRepository>, rbac: Arc<RbacEnforcer>) -> Self {
        Self { storage_repo, rbac }
    }

    /// Lists all available storages.
    pub async fn list_storages(&self, ctx: &RequestContext) -> Result<Vec<Storage>, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::StorageView)?;

        self.storage_repo
            .find_all()
            .await
            .map_err(|e| AppError::internal(format!("Failed to list storages: {e}")))
    }

    /// Gets a specific storage.
    pub async fn get_storage(
        &self,
        ctx: &RequestContext,
        storage_id: Uuid,
    ) -> Result<Storage, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::StorageView)?;

        self.storage_repo
            .find_by_id(storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Storage not found"))
    }

    /// Gets usage statistics for a storage.
    pub async fn get_usage(
        &self,
        ctx: &RequestContext,
        storage_id: Uuid,
    ) -> Result<StorageUsage, AppError> {
        let storage = self.get_storage(ctx, storage_id).await?;

        let (file_count, folder_count) = self
            .storage_repo
            .get_counts(storage_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get counts: {e}")))?;

        let used_bytes = storage.used_bytes.unwrap_or(0);

        let usage_percent = storage.quota_bytes.map(|quota| {
            if quota > 0 {
                (used_bytes as f64 / quota as f64) * 100.0
            } else {
                0.0
            }
        });

        Ok(StorageUsage {
            storage_id: storage.id,
            name: storage.name,
            used_bytes,
            quota_bytes: storage.quota_bytes,
            usage_percent,
            file_count,
            folder_count,
        })
    }
}
