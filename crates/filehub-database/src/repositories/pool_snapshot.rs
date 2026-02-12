//! Pool snapshot repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::license::pool::PoolSnapshot;

/// Repository for license pool snapshots.
#[derive(Debug, Clone)]
pub struct PoolSnapshotRepository {
    pool: PgPool,
}

impl PoolSnapshotRepository {
    /// Create a new pool snapshot repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find the latest snapshot.
    pub async fn find_latest(&self) -> AppResult<Option<PoolSnapshot>> {
        sqlx::query_as::<_, PoolSnapshot>(
            "SELECT * FROM pool_snapshots ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find latest snapshot", e)
        })
    }

    /// Find a snapshot by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<PoolSnapshot>> {
        sqlx::query_as::<_, PoolSnapshot>("SELECT * FROM pool_snapshots WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find snapshot", e))
    }

    /// List recent snapshots.
    pub async fn find_recent(&self, page: &PageRequest) -> AppResult<PageResponse<PoolSnapshot>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pool_snapshots")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count snapshots", e)
            })?;

        let snapshots = sqlx::query_as::<_, PoolSnapshot>(
            "SELECT * FROM pool_snapshots ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list snapshots", e))?;

        Ok(PageResponse::new(
            snapshots,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Create a new snapshot.
    pub async fn create(
        &self,
        total_seats: i32,
        checked_out: i32,
        available: i32,
        admin_reserved: i32,
        active_sessions: i32,
        drift_detected: bool,
        drift_detail: Option<&serde_json::Value>,
        source: &str,
    ) -> AppResult<PoolSnapshot> {
        sqlx::query_as::<_, PoolSnapshot>(
            "INSERT INTO pool_snapshots (total_seats, checked_out, available, admin_reserved, active_sessions, drift_detected, drift_detail, source) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
            .bind(total_seats)
            .bind(checked_out)
            .bind(available)
            .bind(admin_reserved)
            .bind(active_sessions)
            .bind(drift_detected)
            .bind(drift_detail)
            .bind(source)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create snapshot", e))
    }

    /// Clean up old snapshots.
    pub async fn cleanup_old(&self, before: chrono::DateTime<chrono::Utc>) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM pool_snapshots WHERE created_at < $1")
            .bind(before)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to cleanup snapshots", e)
            })?;
        Ok(result.rows_affected())
    }
}
