//! Session, chunk, temp, and version cleanup job handlers.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use tracing;

use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::session::SessionRepository;
use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Handles session cleanup jobs
#[derive(Debug)]
pub struct CleanupJobHandler {
    /// Session repository
    session_repo: Arc<SessionRepository>,
    /// File repository
    file_repo: Arc<FileRepository>,
    /// Data root directory
    data_root: PathBuf,
}

impl CleanupJobHandler {
    /// Create a new cleanup job handler
    pub fn new(
        session_repo: Arc<SessionRepository>,
        file_repo: Arc<FileRepository>,
        data_root: PathBuf,
    ) -> Self {
        Self {
            session_repo,
            file_repo,
            data_root,
        }
    }

    /// Clean up expired sessions
    async fn cleanup_sessions(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running session cleanup");

        let count = self
            .session_repo
            .cleanup_expired(Utc::now())
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Session cleanup failed: {}", e)))?;

        tracing::info!("Cleaned up {} expired sessions", count);

        Ok(serde_json::json!({
            "task": "session_cleanup",
            "expired_sessions_removed": count,
        }))
    }

    /// Clean up expired chunked uploads
    async fn cleanup_chunks(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running chunk cleanup");

        let expired =
            self.file_repo.find_expired_uploads().await.map_err(|e| {
                JobExecutionError::Transient(format!("Chunk cleanup failed: {}", e))
            })?;

        let mut cleaned = 0;
        for upload in &expired {
            let temp_path = PathBuf::from(&upload.temp_path);
            if temp_path.exists() {
                if let Err(e) = tokio::fs::remove_dir_all(&temp_path).await {
                    tracing::warn!("Failed to remove temp dir for upload {}: {}", upload.id, e);
                }
            }
            if let Err(e) = self.file_repo.delete_upload(upload.id).await {
                tracing::warn!("Failed to delete upload record {}: {}", upload.id, e);
            } else {
                cleaned += 1;
            }
        }

        tracing::info!("Cleaned up {} expired chunked uploads", cleaned);

        Ok(serde_json::json!({
            "task": "chunk_cleanup",
            "expired_uploads_removed": cleaned,
        }))
    }

    /// Clean up temporary files
    async fn cleanup_temp(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running temp file cleanup");

        let temp_dir = self.data_root.join("temp");
        let mut removed = 0u64;

        if temp_dir.exists() {
            let cutoff = Utc::now() - Duration::hours(24);
            let mut entries = tokio::fs::read_dir(&temp_dir).await.map_err(|e| {
                JobExecutionError::Transient(format!("Failed to read temp dir: {}", e))
            })?;

            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_dt: chrono::DateTime<Utc> = modified.into();
                        if modified_dt < cutoff {
                            let path = entry.path();
                            let result = if metadata.is_dir() {
                                tokio::fs::remove_dir_all(&path).await
                            } else {
                                tokio::fs::remove_file(&path).await
                            };
                            if let Err(e) = result {
                                tracing::warn!("Failed to remove temp entry {:?}: {}", path, e);
                            } else {
                                removed += 1;
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Cleaned up {} temp files/directories", removed);

        Ok(serde_json::json!({
            "task": "temp_cleanup",
            "items_removed": removed,
        }))
    }

    /// Clean up old file versions beyond retention policy
    async fn cleanup_versions(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running version cleanup");

        let max_versions_per_file = 10;
        let count = self
            .file_repo
            .delete_old_versions(max_versions_per_file)
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Version cleanup failed: {}", e)))?;

        tracing::info!("Cleaned up {} old file versions", count);

        Ok(serde_json::json!({
            "task": "version_cleanup",
            "versions_removed": count,
            "max_versions_per_file": max_versions_per_file,
        }))
    }
}

#[async_trait]
impl JobHandler for CleanupJobHandler {
    fn job_type(&self) -> &str {
        "cleanup"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let task = job
            .payload
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = match task {
            "session_cleanup" => self.cleanup_sessions().await?,
            "chunk_cleanup" => self.cleanup_chunks().await?,
            "temp_cleanup" => self.cleanup_temp().await?,
            "version_cleanup" => self.cleanup_versions().await?,
            _ => {
                return Err(JobExecutionError::Permanent(format!(
                    "Unknown cleanup task: '{}'",
                    task
                )));
            }
        };

        Ok(Some(result))
    }
}

/// Separate handler that matches specific job_type strings dispatched by the scheduler
#[derive(Debug)]
pub struct SessionCleanupHandler {
    /// Inner cleanup handler
    inner: Arc<CleanupJobHandler>,
}

impl SessionCleanupHandler {
    /// Create a new session cleanup handler
    pub fn new(inner: Arc<CleanupJobHandler>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl JobHandler for SessionCleanupHandler {
    fn job_type(&self) -> &str {
        "session_cleanup"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let result = self.inner.cleanup_sessions().await?;
        Ok(Some(result))
    }
}

/// Handler for chunk_cleanup job type
#[derive(Debug)]
pub struct ChunkCleanupHandler {
    /// Inner cleanup handler
    inner: Arc<CleanupJobHandler>,
}

impl ChunkCleanupHandler {
    /// Create a new chunk cleanup handler
    pub fn new(inner: Arc<CleanupJobHandler>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl JobHandler for ChunkCleanupHandler {
    fn job_type(&self) -> &str {
        "chunk_cleanup"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let result = self.inner.cleanup_chunks().await?;
        Ok(Some(result))
    }
}

/// Handler for temp_cleanup job type
#[derive(Debug)]
pub struct TempCleanupHandler {
    /// Inner cleanup handler
    inner: Arc<CleanupJobHandler>,
}

impl TempCleanupHandler {
    /// Create a new temp cleanup handler
    pub fn new(inner: Arc<CleanupJobHandler>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl JobHandler for TempCleanupHandler {
    fn job_type(&self) -> &str {
        "temp_cleanup"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let result = self.inner.cleanup_temp().await?;
        Ok(Some(result))
    }
}

/// Handler for version_cleanup job type
#[derive(Debug)]
pub struct VersionCleanupHandler {
    /// Inner cleanup handler
    inner: Arc<CleanupJobHandler>,
}

impl VersionCleanupHandler {
    /// Create a new version cleanup handler
    pub fn new(inner: Arc<CleanupJobHandler>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl JobHandler for VersionCleanupHandler {
    fn job_type(&self) -> &str {
        "version_cleanup"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let result = self.inner.cleanup_versions().await?;
        Ok(Some(result))
    }
}
