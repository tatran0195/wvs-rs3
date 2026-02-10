//! Configuration management CLI commands.

use clap::{Args, Subcommand};

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;

/// Arguments for config commands
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Config subcommand
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Config subcommands
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Validate configuration file
    Validate,
    /// Generate a default configuration file
    Generate {
        /// Output file path
        #[arg(short, long, default_value = "config/generated.toml")]
        output: String,
    },
}

/// Execute config commands
pub async fn execute(
    args: &ConfigArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    match &args.command {
        ConfigCommand::Show => {
            let config = super::load_config(config_path).await?;
            output::print_item(&config, format);
        }
        ConfigCommand::Validate => match super::load_config(config_path).await {
            Ok(config) => {
                output::print_success(&format!("Configuration '{}' is valid", config_path));
                println!("  Server: {}:{}", config.server.host, config.server.port);
                println!("  Database: {}", mask_password(&config.database.url));
                println!("  Cache: {}", config.cache.provider);
                println!("  Storage: {}", config.storage.default_provider);
            }
            Err(e) => {
                output::print_error(&format!("Configuration invalid: {}", e));
                return Err(e);
            }
        },
        ConfigCommand::Generate { output: out_path } => {
            let default_config = include_str!("../../../../config/default.toml");

            if let Some(parent) = std::path::Path::new(out_path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to create dir: {}", e)))?;
            }

            tokio::fs::write(out_path, default_config)
                .await
                .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

            output::print_success(&format!("Default config written to '{}'", out_path));
        }
    }

    Ok(())
}

/// Mask password in database URL for display
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let scheme_end = url.find("://").map(|i| i + 3).unwrap_or(0);
            if colon_pos > scheme_end {
                let mut masked = url[..colon_pos + 1].to_string();
                masked.push_str("****");
                masked.push_str(&url[at_pos..]);
                return masked;
            }
        }
    }
    url.to_string()
}
