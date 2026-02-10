//! Weekly report generation service.

use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::audit::AuditLogRepository;
use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::user::UserRepository;

/// Generates weekly system usage reports.
#[derive(Debug, Clone)]
pub struct WeeklyReportService {
    /// User repository.
    user_repo: Arc<UserRepository>,
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Audit log repository.
    audit_repo: Arc<AuditLogRepository>,
}

/// Weekly report data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeeklyReport {
    /// Report period start.
    pub period_start: chrono::DateTime<Utc>,
    /// Report period end.
    pub period_end: chrono::DateTime<Utc>,
    /// Total number of users.
    pub total_users: i64,
    /// New users this week.
    pub new_users: i64,
    /// Total files.
    pub total_files: i64,
    /// Files uploaded this week.
    pub files_uploaded: i64,
    /// Total storage used (bytes).
    pub total_storage_bytes: i64,
    /// Number of login events.
    pub login_count: i64,
    /// Number of download events.
    pub download_count: i64,
}

impl WeeklyReportService {
    /// Creates a new weekly report service.
    pub fn new(
        user_repo: Arc<UserRepository>,
        file_repo: Arc<FileRepository>,
        audit_repo: Arc<AuditLogRepository>,
    ) -> Self {
        Self {
            user_repo,
            file_repo,
            audit_repo,
        }
    }

    /// Generates a weekly report for the past 7 days.
    pub async fn generate(&self) -> Result<WeeklyReport, AppError> {
        let now = Utc::now();
        let week_ago = now - Duration::days(7);

        let total_users = self
            .user_repo
            .count_all()
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let new_users = self
            .user_repo
            .count_created_since(week_ago)
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let total_files = self
            .file_repo
            .count_all()
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let files_uploaded = self
            .file_repo
            .count_created_since(week_ago)
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let total_storage_bytes = self
            .file_repo
            .total_size_bytes()
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let login_count = self
            .audit_repo
            .count_actions_since("login", week_ago)
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        let download_count = self
            .audit_repo
            .count_actions_since("file.download", week_ago)
            .await
            .map_err(|e| AppError::internal(format!("Report query failed: {e}")))?;

        Ok(WeeklyReport {
            period_start: week_ago,
            period_end: now,
            total_users,
            new_users,
            total_files,
            files_uploaded,
            total_storage_bytes,
            login_count,
            download_count,
        })
    }
}
