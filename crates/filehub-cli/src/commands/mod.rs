//! CLI command definitions and dispatch.

pub mod admin;
pub mod audit;
pub mod broadcast;
pub mod config;
pub mod folder;
pub mod license;
pub mod migrate;
pub mod serve;
pub mod user;
pub mod worker;

use clap::{Parser, Subcommand};

use crate::output::OutputFormat;
use filehub_core::error::AppError;

/// FileHub — Enterprise File Management Platform
#[derive(Debug, Parser)]
#[command(name = "filehub", version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config/default.toml")]
    pub config: String,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level commands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the FileHub server
    Serve(serve::ServeArgs),
    /// Database migration management
    Migrate(migrate::MigrateArgs),
    /// Admin user management
    Admin(admin::AdminArgs),
    /// User management
    User(user::UserArgs),
    /// Storage management
    /// Folder management
    Folder(folder::FolderArgs),
    /// Configuration management
    Config(config::ConfigArgs),
    /// License management
    License(license::LicenseArgs),
    /// Admin broadcast
    Broadcast(broadcast::BroadcastArgs),
    /// Audit log
    Audit(audit::AuditArgs),
    /// Worker management
    Worker(worker::WorkerArgs),
}

impl Cli {
    /// Execute the CLI command
    pub async fn execute(&self) -> Result<(), AppError> {
        match &self.command {
            Commands::Serve(args) => serve::execute(args, &self.config).await,
            Commands::Migrate(args) => migrate::execute(args, &self.config).await,
            Commands::Admin(args) => admin::execute(args, &self.config, self.format).await,
            Commands::User(args) => user::execute(args, &self.config, self.format).await,
            Commands::Folder(args) => folder::execute(args, &self.config, self.format).await,
            Commands::Config(args) => config::execute(args, &self.config, self.format).await,
            Commands::License(args) => license::execute(args, &self.config, self.format).await,
            Commands::Broadcast(args) => broadcast::execute(args, &self.config).await,
            Commands::Audit(args) => audit::execute(args, &self.config, self.format).await,
            Commands::Worker(args) => worker::execute(args, &self.config).await,
        }
    }
}

/// Helper: load configuration from file
pub async fn load_config(config_path: &str) -> Result<filehub_core::config::AppConfig, AppError> {
    filehub_core::config::AppConfig::load(config_path)
        .map_err(|e| AppError::internal(format!("Failed to load config: {}", e)))
}

/// Helper: create database pool from config
pub async fn create_db_pool(
    config: &filehub_core::config::AppConfig,
) -> Result<sqlx::PgPool, AppError> {
    let pool = filehub_database::connection::DatabasePool::connect(&config.database).await?;
    Ok(pool.into_pool())
}
