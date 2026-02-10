//! Job repository implementation.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::job::model::{CreateJob, Job};
use filehub_entity::job::status::JobStatus;

/// Repository for background job CRUD and queue operations.
#[derive(Debug, Clone)]
pub struct JobRepository {
    pool: PgPool,
}

impl JobRepository {
    /// Create a new job repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a job by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Job>> {
        sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find job", e))
    }

    /// List jobs with pagination.
    pub async fn find_all(&self, page: &PageRequest) -> AppResult<PageResponse<Job>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count jobs", e))?;

        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list jobs", e))?;

        Ok(PageResponse::new(
            jobs,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Fetch the next pending job from a queue (SKIP LOCKED for concurrency).
    pub async fn dequeue(&self, queue: &str, worker_id: &str) -> AppResult<Option<Job>> {
        sqlx::query_as::<_, Job>(
            "UPDATE jobs SET status = 'running', started_at = NOW(), worker_id = $2, \
             attempts = COALESCE(attempts, 0) + 1, updated_at = NOW() \
             WHERE id = ( \
                SELECT id FROM jobs \
                WHERE queue = $1 AND status = 'pending' \
                AND (scheduled_at IS NULL OR scheduled_at <= NOW()) \
                ORDER BY \
                    CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 WHEN 'low' THEN 3 END, \
                    created_at ASC \
                FOR UPDATE SKIP LOCKED \
                LIMIT 1 \
             ) RETURNING *"
        )
            .bind(queue)
            .bind(worker_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to dequeue job", e))
    }

    /// Create a new job.
    pub async fn create(&self, data: &CreateJob) -> AppResult<Job> {
        sqlx::query_as::<_, Job>(
            "INSERT INTO jobs (job_type, queue, priority, payload, max_attempts, scheduled_at, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
        )
            .bind(&data.job_type)
            .bind(&data.queue)
            .bind(&data.priority)
            .bind(&data.payload)
            .bind(data.max_attempts)
            .bind(data.scheduled_at)
            .bind(data.created_by)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create job", e))
    }

    /// Mark a job as completed.
    pub async fn complete(
        &self,
        job_id: Uuid,
        result: Option<&serde_json::Value>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'completed', result = $2, completed_at = NOW(), updated_at = NOW() \
             WHERE id = $1"
        )
            .bind(job_id)
            .bind(result)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to complete job", e))?;
        Ok(())
    }

    /// Mark a job as failed.
    pub async fn fail(&self, job_id: Uuid, error_message: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'failed', error_message = $2, updated_at = NOW() WHERE id = $1"
        )
            .bind(job_id)
            .bind(error_message)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to mark job as failed", e))?;
        Ok(())
    }

    /// Reset a failed job to pending for retry.
    pub async fn retry(&self, job_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'pending', error_message = NULL, started_at = NULL, \
             worker_id = NULL, updated_at = NOW() \
             WHERE id = $1 AND status = 'failed'",
        )
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to retry job", e))?;
        Ok(())
    }

    /// Cancel a job.
    pub async fn cancel(&self, job_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE jobs SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND status IN ('pending', 'queued')"
        )
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to cancel job", e))?;
        Ok(())
    }

    /// Clean up old completed/failed jobs.
    pub async fn cleanup_old(&self, before: DateTime<Utc>) -> AppResult<u64> {
        let result = sqlx::query(
            "DELETE FROM jobs WHERE status IN ('completed', 'failed', 'cancelled') AND updated_at < $1"
        )
            .bind(before)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to cleanup jobs", e))?;
        Ok(result.rows_affected())
    }
}
