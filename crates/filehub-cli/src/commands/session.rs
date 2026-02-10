//! Session management CLI commands.

use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_database::repositories::session::SessionRepository;

/// Arguments for session commands
#[derive(Debug, Args)]
pub struct SessionArgs {
    /// Session subcommand
    #[command(subcommand)]
    pub command: SessionCommand,
}

/// Session subcommands
#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List active sessions
    List,
    /// Kill a specific session
    Kill {
        /// Session ID
        id: String,
    },
    /// Kill all non-admin sessions
    KillAll {
        /// Skip confirmation
        #[arg(long)]
        force: bool,
    },
    /// Count active sessions
    Count,
}

/// Session display row
#[derive(Debug, Serialize, Tabled)]
struct SessionRow {
    /// Session ID
    id: String,
    /// User ID
    user_id: String,
    /// IP Address
    ip: String,
    /// WS Connected
    ws: String,
    /// Last Activity
    last_activity: String,
    /// Expires
    expires: String,
}

/// Execute session commands
pub async fn execute(
    args: &SessionArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let session_repo = SessionRepository::new(pool.clone());

    match &args.command {
        SessionCommand::List => {
            let sessions = session_repo
                .find_active()
                .await
                .map_err(|e| AppError::internal(format!("Failed to list sessions: {}", e)))?;

            let rows: Vec<SessionRow> = sessions
                .iter()
                .map(|s| SessionRow {
                    id: s.id.to_string()[..8].to_string(),
                    user_id: s.user_id.to_string()[..8].to_string(),
                    ip: s.ip_address.clone(),
                    ws: if s.ws_connected { "✓" } else { "✗" }.to_string(),
                    last_activity: s.last_activity.format("%H:%M:%S").to_string(),
                    expires: s.expires_at.format("%H:%M:%S").to_string(),
                })
                .collect();

            output::print_list(&rows, format);
        }
        SessionCommand::Kill { id } => {
            let sid = uuid::Uuid::parse_str(id)
                .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))?;

            session_repo
                .terminate(sid, None, "CLI termination")
                .await
                .map_err(|e| AppError::internal(format!("Failed to kill session: {}", e)))?;

            output::print_success(&format!("Session {} terminated", id));
        }
        SessionCommand::KillAll { force } => {
            if !force {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt("Terminate ALL non-admin sessions?")
                    .default(false)
                    .interact()
                    .map_err(|e| AppError::internal(format!("Input error: {}", e)))?;

                if !confirm {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            let count = session_repo
                .terminate_all_non_admin()
                .await
                .map_err(|e| AppError::internal(format!("Failed to kill sessions: {}", e)))?;

            output::print_success(&format!("Terminated {} sessions", count));
        }
        SessionCommand::Count => {
            let count = session_repo
                .count_active()
                .await
                .map_err(|e| AppError::internal(format!("Failed to count: {}", e)))?;

            println!("Active sessions: {}", count);
        }
    }

    Ok(())
}
