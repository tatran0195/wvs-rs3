//! FileHub Server — Enterprise File Management Platform
//!
//! Main entry point that wires all crates together and starts the server.

use tracing;
use tracing_subscriber::{EnvFilter, fmt};

use filehub_core::config::AppConfig;
use filehub_core::error::AppError;

#[tokio::main]
async fn main() {
    let config = match load_configuration() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    init_logging(&config);

    if let Err(e) = run(config).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Load configuration from file and environment
fn load_configuration() -> Result<AppConfig, AppError> {
    let env = std::env::var("FILEHUB_ENV").unwrap_or_else(|_| "development".to_string());

    tracing::info!("Loading configuration for environment: {}", env);

    AppConfig::load(&env)
}

/// Initialize tracing/logging
fn init_logging(config: &AppConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    match config.logging.format.as_str() {
        "json" => {
            fmt()
                .json()
                .with_env_filter(filter)
                .with_target(true)
                .with_thread_ids(true)
                .init();
        }
        _ => {
            fmt()
                .pretty()
                .with_env_filter(filter)
                .with_target(true)
                .init();
        }
    }
}

/// Main server run function
async fn run(config: AppConfig) -> Result<(), AppError> {
    // ── Step 1: Database connection ──────────────────────────────
    tracing::info!("Connecting to database...");
    let db_pool = filehub_database::DatabasePool::connect(&config.database)
        .await
        .map_err(|e| AppError::internal(format!("Database connection failed: {}", e)))?;

    tracing::info!("Running database migrations...");
    filehub_database::migration::run_migrations(db_pool.pool())
        .await
        .map_err(|e| AppError::internal(format!("Migration failed: {}", e)))?;
    tracing::info!("Database migrations complete");

    // ── Step 2: Delegate to API crate ───────────────────────────
    filehub_api::app::run_server(config, db_pool.into_pool()).await
}
