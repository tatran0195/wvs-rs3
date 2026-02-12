//! Worker management CLI commands.

use clap::{Args, Subcommand};

use crate::output;
use filehub_core::error::AppError;
use filehub_database::repositories::job::JobRepository;

/// Arguments for worker commands
#[derive(Debug, Args)]
pub struct WorkerArgs {
    /// Worker subcommand
    #[command(subcommand)]
    pub command: WorkerCommand,
}

/// Worker subcommands
#[derive(Debug, Subcommand)]
pub enum WorkerCommand {
    /// Show worker/queue status
    Status,
    /// Trigger a specific job type
    Trigger {
        /// Job type to trigger
        job_type: String,
        /// JSON payload
        #[arg(short, long, default_value = "{}")]
        payload: String,
    },
}

/// Execute worker commands
pub async fn execute(args: &WorkerArgs, config_path: &str) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let job_repo = JobRepository::new(pool.clone());

    match &args.command {
        WorkerCommand::Status => {
            let pending = job_repo
                .count_by_status(filehub_entity::job::JobStatus::Pending)
                .await
                .map_err(|e| AppError::internal(format!("Failed to count: {}", e)))?;
            let running = job_repo
                .count_by_status(filehub_entity::job::JobStatus::Running)
                .await
                .map_err(|e| AppError::internal(format!("Failed to count: {}", e)))?;
            let failed = job_repo
                .count_by_status(filehub_entity::job::JobStatus::Failed)
                .await
                .map_err(|e| AppError::internal(format!("Failed to count: {}", e)))?;
            let completed = job_repo
                .count_by_status(filehub_entity::job::JobStatus::Completed)
                .await
                .map_err(|e| AppError::internal(format!("Failed to count: {}", e)))?;

            println!("Worker Queue Status:");
            output::print_kv("Pending", &pending.to_string());
            output::print_kv("Running", &running.to_string());
            output::print_kv("Failed", &failed.to_string());
            output::print_kv("Completed", &completed.to_string());
            output::print_kv("Worker Enabled", &config.worker.enabled.to_string());
            output::print_kv("Concurrency", &config.worker.concurrency.to_string());
        }
        WorkerCommand::Trigger { job_type, payload } => {
            let payload_value: serde_json::Value = serde_json::from_str(payload)
                .map_err(|e| AppError::bad_request(&format!("Invalid JSON payload: {}", e)))?;

            let create_data = filehub_entity::job::model::CreateJob {
                job_type: job_type.clone(),
                queue: "default".to_string(),
                priority: filehub_entity::job::JobPriority::Normal,
                payload: payload_value,
                max_attempts: 3,
                scheduled_at: None,
                created_by: None,
            };

            let job = job_repo
                .create(&create_data)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create job: {}", e)))?;

            output::print_success(&format!("Job '{}' enqueued (id: {})", job_type, job.id));
        }
    }

    Ok(())
}
