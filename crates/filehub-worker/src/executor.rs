//! Job executor — dispatches jobs to registered handlers.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing;

use filehub_core::error::AppError;
use filehub_entity::job::model::Job;

/// Trait for job handler implementations
#[async_trait]
pub trait JobHandler: Send + Sync + std::fmt::Debug {
    /// Get the job type this handler processes
    fn job_type(&self) -> &str;

    /// Execute the job with the given payload
    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError>;
}

/// Error from job execution
#[derive(Debug, thiserror::Error)]
pub enum JobExecutionError {
    /// Permanent failure — do not retry
    #[error("Permanent job failure: {0}")]
    Permanent(String),

    /// Transient failure — may retry
    #[error("Transient job failure: {0}")]
    Transient(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(#[from] AppError),
}

/// Dispatches jobs to the appropriate handler based on job_type
#[derive(Debug)]
pub struct JobExecutor {
    /// Registered job handlers by type
    handlers: HashMap<String, Arc<dyn JobHandler>>,
}

impl JobExecutor {
    /// Create a new job executor
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a job handler
    pub fn register(&mut self, handler: Arc<dyn JobHandler>) {
        let job_type = handler.job_type().to_string();
        tracing::info!("Registered job handler for type '{}'", job_type);
        self.handlers.insert(job_type, handler);
    }

    /// Execute a job by dispatching to the correct handler
    pub async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let handler = self.handlers.get(&job.job_type).ok_or_else(|| {
            JobExecutionError::Permanent(format!(
                "No handler registered for job type '{}'",
                job.job_type
            ))
        })?;

        tracing::info!(
            "Executing job: id={}, type='{}', attempt={}/{}",
            job.id,
            job.job_type,
            job.attempts.unwrap_or(0) + 1,
            job.max_attempts.unwrap_or(0)
        );

        handler.execute(job).await
    }

    /// Check if a handler is registered for a job type
    pub fn has_handler(&self, job_type: &str) -> bool {
        self.handlers.contains_key(job_type)
    }

    /// Get the list of registered job types
    pub fn registered_types(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl Default for JobExecutor {
    fn default() -> Self {
        Self::new()
    }
}
