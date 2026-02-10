//! Job queue abstraction for enqueuing and dequeuing background jobs.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::id::UserId;
use filehub_database::repositories::job::JobRepository;
use filehub_entity::job::model::Job;
use filehub_entity::job::status::{JobPriority, JobStatus};

/// Parameters for creating a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreateParams {
    /// Type of job (e.g., "cad_conversion", "session_cleanup")
    pub job_type: String,
    /// Queue name (e.g., "default", "conversion", "maintenance")
    pub queue: String,
    /// Priority level
    pub priority: JobPriority,
    /// Job payload as JSON
    pub payload: serde_json::Value,
    /// Maximum retry attempts
    pub max_attempts: i32,
    /// Optional scheduled time (run after this time)
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Optional user who created the job
    pub created_by: Option<UserId>,
}

/// Job queue for enqueuing and dequeuing work
#[derive(Debug, Clone)]
pub struct JobQueue {
    /// Job repository for database persistence
    repo: Arc<JobRepository>,
    /// Worker identifier for claiming jobs
    worker_id: String,
}

impl JobQueue {
    /// Create a new job queue
    pub fn new(repo: Arc<JobRepository>, worker_id: String) -> Self {
        Self { repo, worker_id }
    }

    /// Enqueue a new job
    pub async fn enqueue(&self, params: JobCreateParams) -> Result<Job, AppError> {
        let now = Utc::now();
        let job = Job {
            id: Uuid::new_v4(),
            job_type: params.job_type.clone(),
            queue: params.queue.clone(),
            priority: params.priority.clone(),
            payload: params.payload.clone(),
            result: None,
            error_message: None,
            status: JobStatus::Pending,
            attempts: 0,
            max_attempts: params.max_attempts,
            scheduled_at: params.scheduled_at,
            started_at: None,
            completed_at: None,
            created_by: params.created_by.map(|id| *id),
            worker_id: None,
            created_at: now,
            updated_at: now,
        };

        self.repo
            .create(&job)
            .await
            .map_err(|e| AppError::internal(format!("Failed to enqueue job: {}", e)))?;

        tracing::debug!(
            "Enqueued job: id={}, type='{}', queue='{}', priority={:?}",
            job.id,
            job.job_type,
            job.queue,
            job.priority
        );

        Ok(job)
    }

    /// Dequeue the next available job from specified queues
    pub async fn dequeue(&self, queues: &[&str]) -> Result<Option<Job>, AppError> {
        for queue in queues {
            let job = self
                .repo
                .claim_next(queue, &self.worker_id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to dequeue job: {}", e)))?;

            if let Some(job) = job {
                tracing::debug!(
                    "Dequeued job: id={}, type='{}', queue='{}'",
                    job.id,
                    job.job_type,
                    job.queue
                );
                return Ok(Some(job));
            }
        }

        Ok(None)
    }

    /// Mark a job as completed successfully
    pub async fn complete(
        &self,
        job_id: Uuid,
        result: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        self.repo
            .mark_completed(job_id, result)
            .await
            .map_err(|e| AppError::internal(format!("Failed to complete job: {}", e)))?;

        tracing::debug!("Job completed: id={}", job_id);
        Ok(())
    }

    /// Mark a job as failed
    pub async fn fail(&self, job_id: Uuid, error: &str) -> Result<(), AppError> {
        self.repo
            .mark_failed(job_id, error)
            .await
            .map_err(|e| AppError::internal(format!("Failed to mark job as failed: {}", e)))?;

        tracing::debug!("Job failed: id={}, error='{}'", job_id, error);
        Ok(())
    }

    /// Mark a job as cancelled
    pub async fn cancel(&self, job_id: Uuid) -> Result<(), AppError> {
        self.repo
            .mark_cancelled(job_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to cancel job: {}", e)))?;

        tracing::debug!("Job cancelled: id={}", job_id);
        Ok(())
    }

    /// Retry a failed job
    pub async fn retry(&self, job_id: Uuid) -> Result<(), AppError> {
        self.repo
            .retry(job_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to retry job: {}", e)))?;

        tracing::debug!("Job retried: id={}", job_id);
        Ok(())
    }

    /// Get queue statistics
    pub async fn stats(&self) -> Result<QueueStats, AppError> {
        let pending = self
            .repo
            .count_by_status(JobStatus::Pending)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count pending jobs: {}", e)))?;

        let running = self
            .repo
            .count_by_status(JobStatus::Running)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count running jobs: {}", e)))?;

        let failed = self
            .repo
            .count_by_status(JobStatus::Failed)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count failed jobs: {}", e)))?;

        Ok(QueueStats {
            pending,
            running,
            failed,
            worker_id: self.worker_id.clone(),
        })
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Number of pending jobs
    pub pending: i64,
    /// Number of running jobs
    pub running: i64,
    /// Number of failed jobs
    pub failed: i64,
    /// Current worker identifier
    pub worker_id: String,
}
