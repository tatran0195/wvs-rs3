//! Start the FileHub server.

use clap::Args;

use filehub_core::error::AppError;

/// Arguments for the serve command
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Override the server port
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Override the server host
    #[arg(long)]
    pub host: Option<String>,

    /// Run database migrations on startup
    #[arg(long, default_value = "true")]
    pub auto_migrate: bool,
}

/// Execute the serve command
pub async fn execute(args: &ServeArgs, config_path: &str) -> Result<(), AppError> {
    let mut config = super::load_config(config_path).await?;

    if let Some(port) = args.port {
        config.server.port = port;
    }
    if let Some(ref host) = args.host {
        config.server.host = host.clone();
    }

    println!("Starting FileHub server...");
    println!("  Host: {}", config.server.host);
    println!("  Port: {}", config.server.port);

    let pool = super::create_db_pool(&config).await?;

    if args.auto_migrate {
        println!("Running database migrations...");
        filehub_database::migration::run_migrations(&pool)
            .await
            .map_err(|e| AppError::internal(format!("Migration failed: {}", e)))?;
        println!("  Migrations applied successfully.");
    }

    filehub_api::app::run_server(config, pool).await
}
