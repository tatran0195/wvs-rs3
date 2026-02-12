//! License management CLI commands.

use clap::{Args, Subcommand};

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;

/// Arguments for license commands
#[derive(Debug, Args)]
pub struct LicenseArgs {
    /// License subcommand
    #[command(subcommand)]
    pub command: LicenseCommand,
}

/// License subcommands
#[derive(Debug, Subcommand)]
pub enum LicenseCommand {
    /// Show license pool status
    Status,
    /// Show pool snapshot history
    History {
        /// Number of snapshots to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
    /// Release all active checkouts (emergency)
    ReleaseAll {
        /// Skip confirmation
        #[arg(long)]
        force: bool,
    },
}

/// Execute license commands
pub async fn execute(
    args: &LicenseArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let checkout_repo = LicenseCheckoutRepository::new(pool.clone());
    let snapshot_repo = PoolSnapshotRepository::new(pool.clone());

    match &args.command {
        LicenseCommand::Status => {
            let active = checkout_repo
                .count_active()
                .await
                .map_err(|e| AppError::internal(format!("Failed to count active: {}", e)))?;

            let latest = snapshot_repo
                .find_latest()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get snapshot: {}", e)))?;

            println!("License Pool Status:");
            output::print_kv("Active Checkouts", &active.to_string());

            if let Some(snap) = latest {
                output::print_kv("Total Seats", &snap.total_seats.to_string());
                output::print_kv("Available", &snap.available.to_string());
                output::print_kv("Checked Out", &snap.checked_out.to_string());
                output::print_kv("Admin Reserved", &snap.admin_reserved.to_string());
                output::print_kv(
                    "Drift Detected",
                    &snap.drift_detected.unwrap_or(false).to_string(),
                );
                output::print_kv("Last Sync", &snap.created_at.to_rfc3339());
            } else {
                output::print_warning("No pool snapshots available");
            }
        }
        LicenseCommand::History { limit } => {
            // let snapshots = snapshot_repo
            //     .find_recent(*limit)
            //     .await
            //     .map_err(|e| AppError::internal(format!("Failed to get history: {}", e)))?;

            // for snap in &snapshots {
            //     println!(
            //         "  {} | total={} checked_out={} available={} drift={}",
            //         snap.created_at.format("%Y-%m-%d %H:%M:%S"),
            //         snap.total_seats,
            //         snap.checked_out,
            //         snap.available,
            //         snap.drift_detected
            //     );
            // }
        }
        LicenseCommand::ReleaseAll { force } => {
            // if !force {
            //     let confirm = dialoguer::Confirm::new()
            //         .with_prompt("Release ALL active license checkouts? This may disrupt users.")
            //         .default(false)
            //         .interact()
            //         .map_err(|e| AppError::internal(format!("Input error: {}", e)))?;

            //     if !confirm {
            //         println!("Cancelled.");
            //         return Ok(());
            //     }
            // }

            // let count = checkout_repo
            //     .checkin_all()
            //     .await
            //     .map_err(|e| AppError::internal(format!("Failed to release: {}", e)))?;

            // output::print_success(&format!("Released {} license checkouts", count));
        }
    }

    Ok(())
}
