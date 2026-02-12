//! Weekly admin report and storage usage report jobs.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use tracing;

use filehub_database::repositories::audit::AuditLogRepository;
use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::session::SessionRepository;
use filehub_database::repositories::storage::StorageRepository;
use filehub_database::repositories::user::UserRepository;
use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Handles weekly report generation
#[derive(Debug)]
pub struct ReportJobHandler {
    /// User repository
    user_repo: Arc<UserRepository>,
    /// File repository
    file_repo: Arc<FileRepository>,
    /// Storage repository
    storage_repo: Arc<StorageRepository>,
    /// Session repository
    session_repo: Arc<SessionRepository>,
    /// Audit log repository
    audit_repo: Arc<AuditLogRepository>,
}

impl ReportJobHandler {
    /// Create a new report job handler
    pub fn new(
        user_repo: Arc<UserRepository>,
        file_repo: Arc<FileRepository>,
        storage_repo: Arc<StorageRepository>,
        session_repo: Arc<SessionRepository>,
        audit_repo: Arc<AuditLogRepository>,
    ) -> Self {
        Self {
            user_repo,
            file_repo,
            storage_repo,
            session_repo,
            audit_repo,
        }
    }

    /// Generate a weekly report
    async fn generate_weekly_report(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Generating weekly report");

        let now = Utc::now();
        let week_ago = now - Duration::weeks(1);

        let total_users =
            self.user_repo.count_all().await.map_err(|e| {
                JobExecutionError::Transient(format!("Failed to count users: {}", e))
            })?;

        let new_users = self
            .user_repo
            .count_created_since(week_ago)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Failed to count new users: {}", e))
            })?;

        let total_files =
            self.file_repo.count_all().await.map_err(|e| {
                JobExecutionError::Transient(format!("Failed to count files: {}", e))
            })?;

        let files_uploaded = self
            .file_repo
            .count_created_since(week_ago)
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Failed to count uploads: {}", e)))?;

        let total_storage_used = self.storage_repo.total_used_bytes().await.map_err(|e| {
            JobExecutionError::Transient(format!("Failed to get storage usage: {}", e))
        })?;

        let active_sessions = self
            .session_repo
            .find_active_by_user_all()
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Failed to count sessions: {}", e))
            })?;

        let audit_count = self.audit_repo.count_since(week_ago).await.map_err(|e| {
            JobExecutionError::Transient(format!("Failed to count audit entries: {}", e))
        })?;

        let report = serde_json::json!({
            "report_type": "weekly",
            "period": {
                "from": week_ago.to_rfc3339(),
                "to": now.to_rfc3339(),
            },
            "users": {
                "total": total_users,
                "new_this_week": new_users,
            },
            "files": {
                "total": total_files,
                "uploaded_this_week": files_uploaded,
            },
            "storage": {
                "total_used_bytes": total_storage_used,
                "total_used_gb": total_storage_used as f64 / (1024.0 * 1024.0 * 1024.0),
            },
            "sessions": {
                "currently_active": active_sessions,
            },
            "audit": {
                "events_this_week": audit_count,
            },
            "generated_at": now.to_rfc3339(),
        });

        tracing::info!("Weekly report generated successfully");
        Ok(report)
    }

    /// Generate storage usage report
    async fn generate_storage_report(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Generating storage usage report");

        let storages = self.storage_repo.find_all_with_usage().await.map_err(|e| {
            JobExecutionError::Transient(format!("Failed to get storage usage: {}", e))
        })?;

        let storage_entries: Vec<Value> = storages
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_string(),
                    "name": s.name,
                    "provider_type": format!("{:?}", s.provider_type),
                    "used_bytes": s.used_bytes,
                    "quota_bytes": s.quota_bytes,
                    "utilization_percent": s.quota_bytes.map(|q| {
                        if q > 0 { (s.used_bytes.unwrap_or(0) as f64 / q as f64) * 100.0 } else { 0.0 }
                    }),
                })
            })
            .collect();

        let report = serde_json::json!({
            "report_type": "storage_usage",
            "storages": storage_entries,
            "generated_at": Utc::now().to_rfc3339(),
        });

        tracing::info!("Storage report generated successfully");
        Ok(report)
    }
}

#[async_trait]
impl JobHandler for ReportJobHandler {
    fn job_type(&self) -> &str {
        "weekly_report"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let task = job
            .payload
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("weekly_report");

        let result = match task {
            "weekly_report" => self.generate_weekly_report().await?,
            "storage_usage" => self.generate_storage_report().await?,
            _ => {
                return Err(JobExecutionError::Permanent(format!(
                    "Unknown report task: '{}'",
                    task
                )));
            }
        };

        Ok(Some(result))
    }
}
