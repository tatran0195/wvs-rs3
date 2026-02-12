//! Storage repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_entity::storage::model::{CreateStorage, Storage};

/// Repository for storage backend CRUD operations.
#[derive(Debug, Clone)]
pub struct StorageRepository {
    pool: PgPool,
}

impl StorageRepository {
    /// Create a new storage repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a storage by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Storage>> {
        sqlx::query_as::<_, Storage>("SELECT * FROM storages WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find storage", e))
    }

    /// Find the default storage.
    pub async fn find_default(&self) -> AppResult<Option<Storage>> {
        sqlx::query_as::<_, Storage>(
            "SELECT * FROM storages WHERE is_default = TRUE AND status = 'active' LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find default storage", e)
        })
    }

    /// List all storages.
    pub async fn find_all(&self) -> AppResult<Vec<Storage>> {
        sqlx::query_as::<_, Storage>("SELECT * FROM storages ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list storages", e))
    }

    /// Create a new storage.
    pub async fn create(&self, data: &CreateStorage) -> AppResult<Storage> {
        sqlx::query_as::<_, Storage>(
            "INSERT INTO storages (name, description, provider_type, config, is_default, quota_bytes, mount_path, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
            .bind(&data.name)
            .bind(&data.description)
            .bind(&data.provider_type)
            .bind(&data.config)
            .bind(data.is_default)
            .bind(data.quota_bytes)
            .bind(&data.mount_path)
            .bind(data.created_by)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create storage", e))
    }

    /// Update used bytes.
    pub async fn update_used_bytes(&self, storage_id: Uuid, used_bytes: i64) -> AppResult<()> {
        sqlx::query("UPDATE storages SET used_bytes = $2, updated_at = NOW() WHERE id = $1")
            .bind(storage_id)
            .bind(used_bytes)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update used bytes", e)
            })?;
        Ok(())
    }

    /// Increment used bytes atomically.
    pub async fn increment_used_bytes(&self, storage_id: Uuid, bytes: i64) -> AppResult<()> {
        sqlx::query(
            "UPDATE storages SET used_bytes = COALESCE(used_bytes, 0) + $2, updated_at = NOW() WHERE id = $1"
        )
            .bind(storage_id)
            .bind(bytes)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to increment used bytes", e))?;
        Ok(())
    }

    /// Get file and folder counts for a storage.
    pub async fn get_counts(&self, storage_id: Uuid) -> AppResult<(i64, i64)> {
        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE storage_id = $1")
                .bind(storage_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to count files", e)
                })?;

        let folder_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE storage_id = $1")
                .bind(storage_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to count folders", e)
                })?;

        Ok((file_count, folder_count))
    }

    /// Delete a storage.
    pub async fn delete(&self, storage_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM storages WHERE id = $1")
            .bind(storage_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to delete storage", e)
            })?;
        Ok(result.rows_affected() > 0)
    }
    /// Recalculate used bytes for all storages.
    pub async fn recalculate_usage(&self) -> AppResult<u64> {
        // Reset all usage to 0 first (optional, but safer if files were deleted without updating storage)
        // Then sum up file sizes per storage and update.
        // Doing this in a single query if possible is better.

        let result = sqlx::query(
            "UPDATE storages s \
             SET used_bytes = (SELECT COALESCE(SUM(size_bytes), 0) FROM files f WHERE f.storage_id = s.id), \
                 updated_at = NOW()",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to recalculate storage usage", e))?;

        Ok(result.rows_affected())
    }

    /// Get total used bytes across all storages.
    pub async fn total_used_bytes(&self) -> AppResult<i64> {
        let total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(used_bytes), 0) FROM storages")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(
                    ErrorKind::Database,
                    "Failed to calculate total storage usage",
                    e,
                )
            })?;
        Ok(total)
    }

    /// Find all storages with usage data (alias for find_all for now as it includes used_bytes).
    pub async fn find_all_with_usage(&self) -> AppResult<Vec<Storage>> {
        self.find_all().await
    }
}
