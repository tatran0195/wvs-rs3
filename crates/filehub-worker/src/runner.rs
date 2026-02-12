//! Worker runner — main loop that polls for jobs and executes them.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time;
use tracing;

use filehub_core::config::WorkerConfig;

use crate::executor::{JobExecutionError, JobExecutor};
use crate::queue::JobQueue;

/// Main worker runner that polls queues and executes jobs
#[derive(Debug)]
pub struct WorkerRunner {
    /// Job queue for polling
    queue: Arc<JobQueue>,
    /// Job executor for dispatching
    executor: Arc<JobExecutor>,
    /// Worker configuration
    config: WorkerConfig,
    /// Worker identifier
    worker_id: String,
    /// Queues to poll (in priority order)
    queues: Vec<String>,
}

impl WorkerRunner {
    /// Create a new worker runner
    pub fn new(
        queue: Arc<JobQueue>,
        executor: Arc<JobExecutor>,
        config: WorkerConfig,
        worker_id: String,
    ) -> Self {
        Self {
            queue,
            executor,
            config,
            worker_id,
            queues: vec![
                "critical".to_string(),
                "conversion".to_string(),
                "default".to_string(),
                "maintenance".to_string(),
            ],
        }
    }

    /// Set the queues to poll
    pub fn with_queues(mut self, queues: Vec<String>) -> Self {
        self.queues = queues;
        self
    }

    /// Start the worker runner — runs until the cancel signal is received
    pub async fn run(&self, mut cancel: watch::Receiver<bool>) {
        tracing::info!(
            "Worker '{}' started with concurrency={}, poll_interval={}s, queues={:?}",
            self.worker_id,
            self.config.concurrency,
            self.config.poll_interval_seconds,
            self.queues
        );

        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.concurrency as usize,
        ));

        let poll_interval = Duration::from_secs(self.config.poll_interval_seconds);

        loop {
            tokio::select! {
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        tracing::info!("Worker '{}' received shutdown signal", self.worker_id);
                        break;
                    }
                }
                _ = self.poll_and_execute(&semaphore) => {
                    tokio::select! {
                        _ = cancel.changed() => {
                            if *cancel.borrow() {
                                tracing::info!("Worker '{}' shutting down", self.worker_id);
                                break;
                            }
                        }
                        _ = time::sleep(poll_interval) => {}
                    }
                }
            }
        }

        tracing::info!(
            "Worker '{}' waiting for in-flight jobs to complete...",
            self.worker_id
        );

        let max_permits = self.config.concurrency as u32;
        let _ = tokio::time::timeout(Duration::from_secs(30), semaphore.acquire_many(max_permits))
            .await;

        tracing::info!("Worker '{}' shut down complete", self.worker_id);
    }

    /// Poll for a job and execute it if available
    async fn poll_and_execute(&self, semaphore: &Arc<tokio::sync::Semaphore>) {
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::trace!("All worker slots occupied, waiting...");
                return;
            }
        };

        let queue_refs: Vec<&str> = self.queues.iter().map(|s| s.as_str()).collect();

        match self.queue.dequeue(&queue_refs).await {
            Ok(Some(job)) => {
                let queue = Arc::clone(&self.queue);
                let executor = Arc::clone(&self.executor);
                let job_id = job.id;
                let job_type = job.job_type.clone();
                let max_attempts = job.max_attempts;
                let attempts = job.attempts;

                tokio::spawn(async move {
                    let _permit = permit;

                    tracing::info!(
                        "Processing job: id={}, type='{}', attempt={}/{}",
                        job_id,
                        job_type,
                        attempts.unwrap_or(0) + 1,
                        max_attempts.unwrap_or(0)
                    );

                    match executor.execute(&job).await {
                        Ok(result) => {
                            if let Err(e) = queue.complete(job_id, result).await {
                                tracing::error!(
                                    "Failed to mark job {} as completed: {}",
                                    job_id,
                                    e
                                );
                            }
                            tracing::info!("Job {} completed successfully", job_id);
                        }
                        Err(JobExecutionError::Transient(msg)) => {
                            tracing::warn!("Job {} failed (transient): {}", job_id, msg);
                            if attempts.unwrap_or(0) + 1 < max_attempts.unwrap_or(0) {
                                if let Err(e) = queue.retry(job_id).await {
                                    tracing::error!("Failed to retry job {}: {}", job_id, e);
                                }
                            } else {
                                if let Err(e) = queue.fail(job_id, &msg).await {
                                    tracing::error!(
                                        "Failed to mark job {} as failed: {}",
                                        job_id,
                                        e
                                    );
                                }
                            }
                        }
                        Err(JobExecutionError::Permanent(msg)) => {
                            tracing::error!("Job {} failed permanently: {}", job_id, msg);
                            if let Err(e) = queue.fail(job_id, &msg).await {
                                tracing::error!("Failed to mark job {} as failed: {}", job_id, e);
                            }
                        }
                        Err(JobExecutionError::Internal(err)) => {
                            let msg = err.to_string();
                            tracing::error!("Job {} internal error: {}", job_id, msg);
                            if let Err(e) = queue.fail(job_id, &msg).await {
                                tracing::error!("Failed to mark job {} as failed: {}", job_id, e);
                            }
                        }
                    }
                });
            }
            Ok(None) => {
                drop(permit);
                tracing::trace!("No jobs available in queues");
            }
            Err(e) => {
                drop(permit);
                tracing::error!("Failed to dequeue job: {}", e);
            }
        }
    }
}
