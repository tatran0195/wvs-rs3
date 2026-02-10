//! Admin broadcast CLI commands.

use clap::{Args, Subcommand};

use crate::output;
use filehub_core::error::AppError;
use filehub_database::repositories::notification::NotificationRepository;

/// Arguments for broadcast commands
#[derive(Debug, Args)]
pub struct BroadcastArgs {
    /// Broadcast subcommand
    #[command(subcommand)]
    pub command: BroadcastCommand,
}

/// Broadcast subcommands
#[derive(Debug, Subcommand)]
pub enum BroadcastCommand {
    /// Send a broadcast message to all users
    Send {
        /// Title
        #[arg(short, long)]
        title: String,
        /// Message body
        #[arg(short, long)]
        message: String,
        /// Severity: info, warning, critical
        #[arg(short, long, default_value = "info")]
        severity: String,
    },
    /// Show broadcast history
    History {
        /// Number of entries
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
}

/// Execute broadcast commands
pub async fn execute(args: &BroadcastArgs, config_path: &str) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let notif_repo = NotificationRepository::new(pool.clone());

    match &args.command {
        BroadcastCommand::Send {
            title,
            message,
            severity,
        } => {
            let broadcast = filehub_entity::notification::model::AdminBroadcast {
                id: uuid::Uuid::new_v4(),
                admin_id: uuid::Uuid::nil(),
                target: "all".to_string(),
                title: title.clone(),
                message: message.clone(),
                severity: severity.clone(),
                persistent: false,
                action_type: None,
                action_payload: None,
                delivered_count: 0,
                created_at: chrono::Utc::now(),
            };

            notif_repo
                .create_broadcast(&broadcast)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create broadcast: {}", e)))?;

            output::print_success(&format!(
                "Broadcast sent: '{}' (severity: {})",
                title, severity
            ));
        }
        BroadcastCommand::History { limit } => {
            let history = notif_repo
                .find_broadcasts(*limit)
                .await
                .map_err(|e| AppError::internal(format!("Failed to get history: {}", e)))?;

            for b in &history {
                println!(
                    "  {} | [{}] {} - {}",
                    b.created_at.format("%Y-%m-%d %H:%M"),
                    b.severity,
                    b.title,
                    b.message.chars().take(60).collect::<String>()
                );
            }

            if history.is_empty() {
                println!("  No broadcasts found.");
            }
        }
    }

    Ok(())
}
