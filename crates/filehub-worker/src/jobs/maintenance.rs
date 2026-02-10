//! Index rebuild and integrity check jobs.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing;

use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::storage::StorageRepository;
use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Handles maintenance tasks
#[derive(Debug)]
pub struct MaintenanceJobHandler {
    /// File repository
    file_repo: Arc<FileRepository>,
    /// Storage repository
    storage_repo: Arc<StorageRepository>,
}

impl MaintenanceJobHandler {
    /// Create a new maintenance job handler
    pub fn new(file_repo: Arc<FileRepository>, storage_repo: Arc<StorageRepository>) -> Self {
        Self {
            file_repo,
            storage_repo,
        }
    }

    /// Rebuild search indexes
    async fn rebuild_indexes(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Rebuilding search indexes");

        self.file_repo
            .rebuild_search_index()
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Index rebuild failed: {}", e)))?;

        tracing::info!("Search indexes rebuilt successfully");

        Ok(serde_json::json!({
            "task": "rebuild_indexes",
            "status": "completed",
        }))
    }

    /// Run integrity checks on file records vs storage
    async fn integrity_check(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running integrity check");

        let orphaned_records =
            self.file_repo.find_orphaned_records().await.map_err(|e| {
                JobExecutionError::Transient(format!("Integrity check failed: {}", e))
            })?;

        let storage_usage = self.storage_repo.recalculate_usage().await.map_err(|e| {
            JobExecutionError::Transient(format!("Storage recalculation failed: {}", e))
        })?;

        tracing::info!(
            "Integrity check complete: {} orphaned records found, storage usage recalculated",
            orphaned_records
        );

        Ok(serde_json::json!({
            "task": "integrity_check",
            "orphaned_records": orphaned_records,
            "storage_usage_recalculated": storage_usage,
        }))
    }
}

#[async_trait]
impl JobHandler for MaintenanceJobHandler {
    fn job_type(&self) -> &str {
        "maintenance"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let task = job
            .payload
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = match task {
            "rebuild_indexes" => self.rebuild_indexes().await?,
            "integrity_check" => self.integrity_check().await?,
            _ => {
                return Err(JobExecutionError::Permanent(format!(
                    "Unknown maintenance task: '{}'",
                    task
                )));
            }
        };

        Ok(Some(result))
    }
}
