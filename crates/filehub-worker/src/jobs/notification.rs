//! Notification cleanup, digest, and broadcast cleanup jobs.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use tracing;

use filehub_database::repositories::notification::NotificationRepository;
use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Handles notification maintenance jobs
#[derive(Debug)]
pub struct NotificationJobHandler {
    /// Notification repository
    notification_repo: Arc<NotificationRepository>,
    /// Max age in days before cleanup
    cleanup_after_days: i64,
    /// Max stored per user
    max_stored_per_user: i64,
}

impl NotificationJobHandler {
    /// Create a new notification job handler
    pub fn new(
        notification_repo: Arc<NotificationRepository>,
        cleanup_after_days: i64,
        max_stored_per_user: i64,
    ) -> Self {
        Self {
            notification_repo,
            cleanup_after_days,
            max_stored_per_user,
        }
    }

    /// Clean up old notifications
    async fn cleanup_notifications(&self) -> Result<Value, JobExecutionError> {
        tracing::info!(
            "Running notification cleanup (older than {} days)",
            self.cleanup_after_days
        );

        let cutoff = Utc::now() - Duration::days(self.cleanup_after_days);

        let expired_count = self
            .notification_repo
            .delete_expired(cutoff)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Notification cleanup failed: {}", e))
            })?;

        let overflow_count = self
            .notification_repo
            .trim_per_user(self.max_stored_per_user)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Notification per-user trim failed: {}", e))
            })?;

        tracing::info!(
            "Notification cleanup: removed {} expired, {} overflow",
            expired_count,
            overflow_count
        );

        Ok(serde_json::json!({
            "task": "notification_cleanup",
            "expired_removed": expired_count,
            "overflow_removed": overflow_count,
            "cutoff_days": self.cleanup_after_days,
            "max_per_user": self.max_stored_per_user,
        }))
    }

    /// Clean up old broadcast records
    async fn cleanup_broadcasts(&self) -> Result<Value, JobExecutionError> {
        tracing::info!("Running broadcast cleanup");

        let cutoff = Utc::now() - Duration::days(90);

        let count = self
            .notification_repo
            .delete_old_broadcasts(cutoff)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Broadcast cleanup failed: {}", e))
            })?;

        tracing::info!("Broadcast cleanup: removed {} old broadcasts", count);

        Ok(serde_json::json!({
            "task": "broadcast_cleanup",
            "removed": count,
        }))
    }
}

#[async_trait]
impl JobHandler for NotificationJobHandler {
    fn job_type(&self) -> &str {
        "notification_cleanup"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let task = job
            .payload
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("notification_cleanup");

        let result = match task {
            "notification_cleanup" => self.cleanup_notifications().await?,
            "broadcast_cleanup" => self.cleanup_broadcasts().await?,
            _ => {
                return Err(JobExecutionError::Permanent(format!(
                    "Unknown notification task: '{}'",
                    task
                )));
            }
        };

        Ok(Some(result))
    }
}
