//! Share repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::share::model::{CreateShare, Share};

/// Repository for share CRUD and token lookup operations.
#[derive(Debug, Clone)]
pub struct ShareRepository {
    pool: PgPool,
}

impl ShareRepository {
    /// Create a new share repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a share by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Share>> {
        sqlx::query_as::<_, Share>("SELECT * FROM shares WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find share", e))
    }

    /// Find a share by token.
    pub async fn find_by_token(&self, token: &str) -> AppResult<Option<Share>> {
        sqlx::query_as::<_, Share>("SELECT * FROM shares WHERE token = $1 AND is_active = TRUE")
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find share by token", e)
            })
    }

    /// List shares created by a user.
    pub async fn find_by_creator(
        &self,
        user_id: Uuid,
        page: &PageRequest,
    ) -> AppResult<PageResponse<Share>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shares WHERE created_by = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count shares", e))?;

        let shares = sqlx::query_as::<_, Share>(
            "SELECT * FROM shares WHERE created_by = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
            .bind(user_id)
            .bind(page.limit() as i64)
            .bind(page.offset() as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list shares", e))?;

        Ok(PageResponse::new(
            shares,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// List shares shared with a user.
    pub async fn find_shared_with_user(
        &self,
        user_id: Uuid,
        page: &PageRequest,
    ) -> AppResult<PageResponse<Share>> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM shares WHERE shared_with = $1 AND is_active = TRUE",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to count shared-with", e)
        })?;

        let shares = sqlx::query_as::<_, Share>(
            "SELECT * FROM shares WHERE shared_with = $1 AND is_active = TRUE \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list shared-with", e))?;

        Ok(PageResponse::new(
            shares,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Create a new share.
    pub async fn create(&self, data: &CreateShare) -> AppResult<Share> {
        sqlx::query_as::<_, Share>(
            "INSERT INTO shares (share_type, resource_type, resource_id, created_by, token, password_hash, \
             shared_with, permission, allow_download, max_downloads, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *"
        )
            .bind(&data.share_type)
            .bind(&data.resource_type)
            .bind(data.resource_id)
            .bind(data.created_by)
            .bind(&data.token)
            .bind(&data.password_hash)
            .bind(data.shared_with)
            .bind(&data.permission)
            .bind(data.allow_download)
            .bind(data.max_downloads)
            .bind(data.expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create share", e))
    }

    /// Increment download count.
    pub async fn increment_download_count(&self, share_id: Uuid) -> AppResult<i32> {
        let row: (i32,) = sqlx::query_as(
            "UPDATE shares SET download_count = COALESCE(download_count, 0) + 1, last_accessed = NOW() \
             WHERE id = $1 RETURNING download_count"
        )
            .bind(share_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to increment downloads", e))?;
        Ok(row.0)
    }

    /// Deactivate a share.
    pub async fn deactivate(&self, share_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("UPDATE shares SET is_active = FALSE WHERE id = $1")
            .bind(share_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to deactivate share", e)
            })?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a share.
    pub async fn delete(&self, share_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM shares WHERE id = $1")
            .bind(share_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to delete share", e))?;
        Ok(result.rows_affected() > 0)
    }
}
