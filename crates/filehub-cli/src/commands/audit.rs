//! Audit log CLI commands.

use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_core::types::pagination::PageRequest;
use filehub_database::repositories::audit::AuditLogRepository;

/// Arguments for audit commands
#[derive(Debug, Args)]
pub struct AuditArgs {
    /// Audit subcommand
    #[command(subcommand)]
    pub command: AuditCommand,
}

/// Audit subcommands
#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    /// Search audit log
    Search {
        /// Filter by action
        #[arg(short, long)]
        action: Option<String>,
        /// Filter by actor (user ID)
        #[arg(long)]
        actor: Option<String>,
        /// Number of results
        #[arg(short, long, default_value = "50")]
        limit: i64,
    },
    /// Export audit log to JSON file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "audit_export.json")]
        output: String,
        /// Days of history to export
        #[arg(short, long, default_value = "30")]
        days: i64,
    },
}

/// Audit display row
#[derive(Debug, Serialize, Tabled)]
struct AuditRow {
    /// Time
    time: String,
    /// Actor ID
    actor: String,
    /// Action
    action: String,
    /// Target type
    target_type: String,
    /// IP
    ip: String,
}

/// Execute audit commands
pub async fn execute(
    args: &AuditArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let audit_repo = AuditLogRepository::new(pool.clone());

    match &args.command {
        AuditCommand::Search {
            action,
            actor,
            limit,
        } => {
            let actor_id = actor
                .as_ref()
                .map(|a| {
                    uuid::Uuid::parse_str(a)
                        .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))
                })
                .transpose()?;

            let page = PageRequest::new(1, *limit as u64);
            let response = audit_repo
                .search(actor_id, action.as_deref(), None, None, &page)
                .await
                .map_err(|e| AppError::internal(format!("Failed to search audit: {}", e)))?;

            let entries = response.items;

            let rows: Vec<AuditRow> = entries
                .iter()
                .map(|e| AuditRow {
                    time: e.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    actor: e.actor_id.to_string()[..8].to_string(),
                    action: e.action.clone(),
                    target_type: e.target_type.clone(),
                    ip: e
                        .ip_address
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                })
                .collect();

            output::print_list(&rows, format);
        }
        AuditCommand::Export {
            output: out_path,
            days,
        } => {
            let since = chrono::Utc::now() - chrono::Duration::days(*days);
            let entries = audit_repo
                .find_since(since)
                .await
                .map_err(|e| AppError::internal(format!("Failed to export audit: {}", e)))?;

            let json = serde_json::to_string_pretty(&entries)
                .map_err(|e| AppError::internal(format!("Serialization error: {}", e)))?;

            tokio::fs::write(out_path, json)
                .await
                .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))?;

            output::print_success(&format!(
                "Exported {} audit entries to '{}'",
                entries.len(),
                out_path
            ));
        }
    }

    Ok(())
}
