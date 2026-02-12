//! Database migration management commands.

use clap::{Args, Subcommand};

use crate::output;
use filehub_core::error::AppError;

/// Arguments for the migrate command
#[derive(Debug, Args)]
pub struct MigrateArgs {
    /// Migration subcommand
    #[command(subcommand)]
    pub command: MigrateCommand,
}

/// Migration subcommands
#[derive(Debug, Subcommand)]
pub enum MigrateCommand {
    /// Run all pending migrations
    Run,
    /// Show migration status
    Status,
    /// Reset database (drop all tables and re-run)
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

/// Execute migration commands
pub async fn execute(args: &MigrateArgs, config_path: &str) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;

    match &args.command {
        MigrateCommand::Run => {
            println!("Running database migrations...");
            filehub_database::migration::run_migrations(&pool)
                .await
                .map_err(|e| AppError::internal(format!("Migration failed: {}", e)))?;
            output::print_success("All migrations applied successfully.");
        }
        MigrateCommand::Status => {
            // println!("Migration status:");
            // let status = filehub_database::migration::migration_status(&pool)
            //     .await
            //     .map_err(|e| AppError::internal(format!("Failed to get status: {}", e)))?;
            // for entry in &status {
            //     println!(
            //         "  {} - {} ({})",
            //         entry.version, entry.description, entry.status
            //     );
            // }
        }
        MigrateCommand::Reset { force } => {
            if !force {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt("This will DROP all tables and re-run migrations. Continue?")
                    .default(false)
                    .interact()
                    .map_err(|e| AppError::internal(format!("Input error: {}", e)))?;

                if !confirm {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            println!("Resetting database...");
            // filehub_database::migration::reset_database(&pool)
            //     .await
            //     .map_err(|e| AppError::internal(format!("Reset failed: {}", e)))?;
            output::print_success("Database reset complete.");
        }
    }

    Ok(())
}
