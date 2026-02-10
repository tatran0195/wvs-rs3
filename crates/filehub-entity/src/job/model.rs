//! Job entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::status::{JobPriority, JobStatus};

/// A background job.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    /// Unique job identifier.
    pub id: Uuid,
    /// Job type identifier (e.g., `"file_assembly"`, `"cad_conversion"`).
    pub job_type: String,
    /// Queue name.
    pub queue: String,
    /// Job priority.
    pub priority: JobPriority,
    /// Job-specific payload (JSON).
    pub payload: serde_json::Value,
    /// Result data on completion (JSON).
    pub result: Option<serde_json::Value>,
    /// Error message on failure.
    pub error_message: Option<String>,
    /// Current job status.
    pub status: JobStatus,
    /// Number of execution attempts.
    pub attempts: Option<i32>,
    /// Maximum allowed attempts.
    pub max_attempts: Option<i32>,
    /// Scheduled execution time (None = immediate).
    pub scheduled_at: Option<DateTime<Utc>>,
    /// When the job started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// User who created the job.
    pub created_by: Option<Uuid>,
    /// Worker ID that picked up the job.
    pub worker_id: Option<String>,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// When the job was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// Check if the job can be retried.
    pub fn can_retry(&self) -> bool {
        let attempts = self.attempts.unwrap_or(0);
        let max = self.max_attempts.unwrap_or(3);
        self.status.can_retry() && attempts < max
    }
}

/// Data required to create a new job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJob {
    /// Job type identifier.
    pub job_type: String,
    /// Queue name.
    pub queue: String,
    /// Priority.
    pub priority: JobPriority,
    /// Job-specific payload.
    pub payload: serde_json::Value,
    /// Maximum retry attempts.
    pub max_attempts: i32,
    /// Scheduled execution time.
    pub scheduled_at: Option<DateTime<Utc>>,
    /// User who created the job.
    pub created_by: Option<Uuid>,
}
