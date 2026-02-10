//! Cron scheduler for periodic maintenance tasks.

use std::sync::Arc;

use tokio::sync::watch;
use tokio_cron_scheduler::{Job as CronJob, JobScheduler, JobSchedulerError};
use tracing;

use filehub_core::error::AppError;

use crate::queue::{JobCreateParams, JobQueue};
use filehub_entity::job::status::JobPriority;

/// Cron-based scheduler for periodic background tasks
pub struct CronScheduler {
    /// The underlying job scheduler
    scheduler: JobScheduler,
    /// Job queue for enqueuing scheduled work
    queue: Arc<JobQueue>,
}

impl std::fmt::Debug for CronScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CronScheduler").finish()
    }
}

impl CronScheduler {
    /// Create a new cron scheduler
    pub async fn new(queue: Arc<JobQueue>) -> Result<Self, AppError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| AppError::internal(format!("Failed to create scheduler: {}", e)))?;

        Ok(Self { scheduler, queue })
    }

    /// Register all default scheduled tasks
    pub async fn register_default_tasks(&self) -> Result<(), AppError> {
        self.register_session_cleanup().await?;
        self.register_chunk_cleanup().await?;
        self.register_temp_cleanup().await?;
        self.register_version_cleanup().await?;
        self.register_weekly_report().await?;
        self.register_pool_sync().await?;
        self.register_presence_reconciliation().await?;
        self.register_notification_cleanup().await?;
        self.register_idle_session_check().await?;

        tracing::info!("All scheduled tasks registered");
        Ok(())
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<(), AppError> {
        self.scheduler
            .start()
            .await
            .map_err(|e| AppError::internal(format!("Failed to start scheduler: {}", e)))?;

        tracing::info!("Cron scheduler started");
        Ok(())
    }

    /// Shutdown the scheduler
    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("Failed to shutdown scheduler: {}", e)))?;

        tracing::info!("Cron scheduler shut down");
        Ok(())
    }

    /// Session cleanup — every 15 minutes
    async fn register_session_cleanup(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 */15 * * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling session cleanup job");
                let params = JobCreateParams {
                    job_type: "session_cleanup".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Normal,
                    payload: serde_json::json!({"task": "session_cleanup"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue session_cleanup: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!("Failed to create session_cleanup schedule: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add session_cleanup schedule: {}", e))
        })?;

        tracing::info!("Registered: session_cleanup (every 15min)");
        Ok(())
    }

    /// Chunk cleanup — every hour
    async fn register_chunk_cleanup(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 0 * * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling chunk cleanup job");
                let params = JobCreateParams {
                    job_type: "chunk_cleanup".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Low,
                    payload: serde_json::json!({"task": "chunk_cleanup"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue chunk_cleanup: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!("Failed to create chunk_cleanup schedule: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add chunk_cleanup schedule: {}", e))
        })?;

        tracing::info!("Registered: chunk_cleanup (every hour)");
        Ok(())
    }

    /// Temp file cleanup — every day at 3 AM
    async fn register_temp_cleanup(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 0 3 * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling temp cleanup job");
                let params = JobCreateParams {
                    job_type: "temp_cleanup".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Low,
                    payload: serde_json::json!({"task": "temp_cleanup"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue temp_cleanup: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!("Failed to create temp_cleanup schedule: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add temp_cleanup schedule: {}", e))
        })?;

        tracing::info!("Registered: temp_cleanup (daily at 3AM)");
        Ok(())
    }

    /// Version cleanup — every Sunday at 4 AM
    async fn register_version_cleanup(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 0 4 * * 0", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling version cleanup job");
                let params = JobCreateParams {
                    job_type: "version_cleanup".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Low,
                    payload: serde_json::json!({"task": "version_cleanup"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue version_cleanup: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!("Failed to create version_cleanup schedule: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add version_cleanup schedule: {}", e))
        })?;

        tracing::info!("Registered: version_cleanup (weekly Sunday 4AM)");
        Ok(())
    }

    /// Weekly report — Monday at 8 AM
    async fn register_weekly_report(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 0 8 * * 1", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling weekly report job");
                let params = JobCreateParams {
                    job_type: "weekly_report".to_string(),
                    queue: "default".to_string(),
                    priority: JobPriority::Normal,
                    payload: serde_json::json!({"task": "weekly_report"}),
                    max_attempts: 3,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue weekly_report: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!("Failed to create weekly_report schedule: {}", e))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add weekly_report schedule: {}", e))
        })?;

        tracing::info!("Registered: weekly_report (Monday 8AM)");
        Ok(())
    }

    /// Pool sync — every 15 seconds
    async fn register_pool_sync(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("*/15 * * * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::trace!("Scheduling pool sync job");
                let params = JobCreateParams {
                    job_type: "pool_sync".to_string(),
                    queue: "critical".to_string(),
                    priority: JobPriority::High,
                    payload: serde_json::json!({"task": "pool_sync"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue pool_sync: {}", e);
                }
            })
        })
        .map_err(|e| AppError::internal(format!("Failed to create pool_sync schedule: {}", e)))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| AppError::internal(format!("Failed to add pool_sync schedule: {}", e)))?;

        tracing::info!("Registered: pool_sync (every 15s)");
        Ok(())
    }

    /// Presence reconciliation — every minute
    async fn register_presence_reconciliation(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 * * * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::trace!("Scheduling presence reconciliation job");
                let params = JobCreateParams {
                    job_type: "presence_reconciliation".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Normal,
                    payload: serde_json::json!({"task": "presence_reconciliation"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue presence_reconciliation: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!(
                "Failed to create presence_reconciliation schedule: {}",
                e
            ))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!(
                "Failed to add presence_reconciliation schedule: {}",
                e
            ))
        })?;

        tracing::info!("Registered: presence_reconciliation (every 1min)");
        Ok(())
    }

    /// Notification cleanup — daily at 2 AM
    async fn register_notification_cleanup(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 0 2 * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling notification cleanup job");
                let params = JobCreateParams {
                    job_type: "notification_cleanup".to_string(),
                    queue: "maintenance".to_string(),
                    priority: JobPriority::Low,
                    payload: serde_json::json!({"task": "notification_cleanup"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue notification_cleanup: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!(
                "Failed to create notification_cleanup schedule: {}",
                e
            ))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!(
                "Failed to add notification_cleanup schedule: {}",
                e
            ))
        })?;

        tracing::info!("Registered: notification_cleanup (daily at 2AM)");
        Ok(())
    }

    /// Idle session check — every 5 minutes
    async fn register_idle_session_check(&self) -> Result<(), AppError> {
        let queue = Arc::clone(&self.queue);
        let job = CronJob::new_async("0 */5 * * * *", move |_uuid, _lock| {
            let queue = Arc::clone(&queue);
            Box::pin(async move {
                tracing::debug!("Scheduling idle session check job");
                let params = JobCreateParams {
                    job_type: "idle_session_check".to_string(),
                    queue: "default".to_string(),
                    priority: JobPriority::Normal,
                    payload: serde_json::json!({"task": "idle_session_check"}),
                    max_attempts: 1,
                    scheduled_at: None,
                    created_by: None,
                };
                if let Err(e) = queue.enqueue(params).await {
                    tracing::error!("Failed to enqueue idle_session_check: {}", e);
                }
            })
        })
        .map_err(|e| {
            AppError::internal(format!(
                "Failed to create idle_session_check schedule: {}",
                e
            ))
        })?;

        self.scheduler.add(job).await.map_err(|e| {
            AppError::internal(format!("Failed to add idle_session_check schedule: {}", e))
        })?;

        tracing::info!("Registered: idle_session_check (every 5min)");
        Ok(())
    }
}
