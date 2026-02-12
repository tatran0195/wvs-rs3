//! Admin user management commands.

use clap::{Args, Subcommand};
use sqlx::PgPool;

use filehub_auth::password::hasher::PasswordHasher;
use filehub_core::error::AppError;
use filehub_database::repositories::user::UserRepository;
use filehub_entity::user::model::CreateUser;
use filehub_entity::user::role::UserRole;

use crate::output;
use crate::output::OutputFormat;

/// Arguments for admin commands
#[derive(Debug, Args)]
pub struct AdminArgs {
    /// Admin subcommand
    #[command(subcommand)]
    pub command: AdminCommand,
}

/// Admin subcommands
#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    /// Create a new admin user
    Create {
        /// Username
        #[arg(short, long)]
        username: Option<String>,
        /// Email
        #[arg(short, long)]
        email: Option<String>,
        /// Password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Reset an admin user's password
    ResetPassword {
        /// Username of the admin
        #[arg(short, long)]
        username: String,
        /// New password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },
}

/// Execute admin commands
pub async fn execute(
    args: &AdminArgs,
    config_path: &str,
    _format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool: PgPool = super::create_db_pool(&config).await?;
    let user_repo = UserRepository::new(pool.clone());
    let hasher = PasswordHasher::new();

    match &args.command {
        AdminCommand::Create {
            username,
            email,
            password,
        } => {
            let username = match username {
                Some(u) => u.clone(),
                None => dialoguer::Input::new()
                    .with_prompt("Admin username")
                    .interact_text()
                    .map_err(|e| AppError::internal(format!("Input error: {}", e)))?,
            };

            let email = match email {
                Some(e) => Some(e.clone()),
                None => {
                    let e: String = dialoguer::Input::new()
                        .with_prompt("Admin email (optional, press Enter to skip)")
                        .allow_empty(true)
                        .interact_text()
                        .map_err(|e| AppError::internal(format!("Input error: {}", e)))?;
                    if e.is_empty() { None } else { Some(e) }
                }
            };

            let password = match password {
                Some(p) => p.clone(),
                None => dialoguer::Password::new()
                    .with_prompt("Admin password")
                    .with_confirmation("Confirm password", "Passwords do not match")
                    .interact()
                    .map_err(|e| AppError::internal(format!("Input error: {}", e)))?,
            };

            let password_hash = hasher
                .hash_password(&password)
                .map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))?;

            let create_user = CreateUser {
                username: username.clone(),
                email,
                password_hash,
                display_name: Some(username.clone()),
                role: UserRole::Admin,
                created_by: None,
            };

            let user = user_repo
                .create(&create_user)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create admin: {}", e)))?;

            output::print_success(&format!(
                "Admin user '{}' created (id: {})",
                username, user.id
            ));
        }
        AdminCommand::ResetPassword { username, password } => {
            let user = user_repo
                .find_by_username(username)
                .await
                .map_err(|e| AppError::internal(format!("Failed to find user: {}", e)))?
                .ok_or_else(|| AppError::not_found(&format!("User '{}' not found", username)))?;

            let password = match password {
                Some(p) => p.clone(),
                None => dialoguer::Password::new()
                    .with_prompt("New password")
                    .with_confirmation("Confirm password", "Passwords do not match")
                    .interact()
                    .map_err(|e| AppError::internal(format!("Input error: {}", e)))?,
            };

            let password_hash = hasher
                .hash_password(&password)
                .map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))?;

            user_repo
                .update_password(user.id, &password_hash)
                .await
                .map_err(|e| AppError::internal(format!("Failed to reset password: {}", e)))?;

            output::print_success(&format!("Password reset for user '{}'", username));
        }
    }

    Ok(())
}
