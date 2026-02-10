//! User management CLI commands.

use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_database::repositories::user::UserRepository;

/// Arguments for user commands
#[derive(Debug, Args)]
pub struct UserArgs {
    /// User subcommand
    #[command(subcommand)]
    pub command: UserCommand,
}

/// User subcommands
#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// List all users
    List {
        /// Filter by role
        #[arg(short, long)]
        role: Option<String>,
    },
    /// Enable a user
    Enable {
        /// Username
        username: String,
    },
    /// Disable a user
    Disable {
        /// Username
        username: String,
    },
}

/// User display row for table output
#[derive(Debug, Serialize, Tabled)]
struct UserRow {
    /// User ID
    id: String,
    /// Username
    username: String,
    /// Email
    email: String,
    /// Role
    role: String,
    /// Status
    status: String,
    /// Created at
    created_at: String,
}

/// Execute user commands
pub async fn execute(
    args: &UserArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let user_repo = UserRepository::new(pool.clone());

    match &args.command {
        UserCommand::List { role } => {
            let users = user_repo
                .find_all_filtered(role.as_deref())
                .await
                .map_err(|e| AppError::internal(format!("Failed to list users: {}", e)))?;

            let rows: Vec<UserRow> = users
                .iter()
                .map(|u| UserRow {
                    id: u.id.to_string(),
                    username: u.username.clone(),
                    email: u.email.clone().unwrap_or_default(),
                    role: format!("{:?}", u.role),
                    status: format!("{:?}", u.status),
                    created_at: u.created_at.format("%Y-%m-%d %H:%M").to_string(),
                })
                .collect();

            output::print_list(&rows, format);
        }
        UserCommand::Enable { username } => {
            let user = user_repo
                .find_by_username(username)
                .await
                .map_err(|e| AppError::internal(format!("Failed to find user: {}", e)))?
                .ok_or_else(|| AppError::not_found(&format!("User '{}' not found", username)))?;

            user_repo
                .update_status(user.id, filehub_entity::user::status::UserStatus::Active)
                .await
                .map_err(|e| AppError::internal(format!("Failed to enable user: {}", e)))?;

            output::print_success(&format!("User '{}' enabled", username));
        }
        UserCommand::Disable { username } => {
            let user = user_repo
                .find_by_username(username)
                .await
                .map_err(|e| AppError::internal(format!("Failed to find user: {}", e)))?
                .ok_or_else(|| AppError::not_found(&format!("User '{}' not found", username)))?;

            user_repo
                .update_status(user.id, filehub_entity::user::status::UserStatus::Inactive)
                .await
                .map_err(|e| AppError::internal(format!("Failed to disable user: {}", e)))?;

            output::print_success(&format!("User '{}' disabled", username));
        }
    }

    Ok(())
}
