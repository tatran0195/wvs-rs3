//! FileHub CLI entry point.

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod commands;
mod output;

use commands::Cli;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    if let Err(e) = cli.execute().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
