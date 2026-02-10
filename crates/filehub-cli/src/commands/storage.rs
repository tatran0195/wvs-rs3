//! Storage management CLI commands.

use clap::{Args, Subcommand};
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_database::repositories::storage::StorageRepository;

/// Arguments for storage commands
#[derive(Debug, Args)]
pub struct StorageArgs {
    /// Storage subcommand
    #[command(subcommand)]
    pub command: StorageCommand,
}

/// Storage subcommands
#[derive(Debug, Subcommand)]
pub enum StorageCommand {
    /// List all storages
    List,
    /// Add a new local storage
    Add {
        /// Storage name
        #[arg(short, long)]
        name: String,
        /// Root path
        #[arg(short, long)]
        path: String,
        /// Set as default
        #[arg(long)]
        default: bool,
    },
    /// Test storage connection
    Test {
        /// Storage ID
        id: String,
    },
}

/// Storage display row
#[derive(Debug, Serialize, Tabled)]
struct StorageRow {
    /// Storage ID
    id: String,
    /// Name
    name: String,
    /// Provider type
    provider: String,
    /// Status
    status: String,
    /// Used bytes
    used: String,
    /// Quota
    quota: String,
    /// Default
    default: String,
}

/// Execute storage commands
pub async fn execute(
    args: &StorageArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let storage_repo = StorageRepository::new(pool.clone());

    match &args.command {
        StorageCommand::List => {
            let storages = storage_repo
                .find_all()
                .await
                .map_err(|e| AppError::internal(format!("Failed to list storages: {}", e)))?;

            let rows: Vec<StorageRow> = storages
                .iter()
                .map(|s| StorageRow {
                    id: s.id.to_string(),
                    name: s.name.clone(),
                    provider: format!("{:?}", s.provider_type),
                    status: format!("{:?}", s.status),
                    used: format_bytes(s.used_bytes),
                    quota: s
                        .quota_bytes
                        .map(format_bytes)
                        .unwrap_or_else(|| "unlimited".to_string()),
                    default: if s.is_default { "✓" } else { "" }.to_string(),
                })
                .collect();

            output::print_list(&rows, format);
        }
        StorageCommand::Add {
            name,
            path,
            default,
        } => {
            let storage = filehub_entity::storage::model::Storage {
                id: uuid::Uuid::new_v4(),
                name: name.clone(),
                description: None,
                provider_type: filehub_entity::storage::provider::StorageProviderType::Local,
                config: serde_json::json!({"root_path": path}),
                status: filehub_entity::storage::model::StorageStatus::Active,
                is_default: *default,
                quota_bytes: None,
                used_bytes: 0,
                mount_path: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                last_synced_at: None,
                created_by: None,
            };

            storage_repo
                .create(&storage)
                .await
                .map_err(|e| AppError::internal(format!("Failed to add storage: {}", e)))?;

            tokio::fs::create_dir_all(path)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))?;

            output::print_success(&format!(
                "Storage '{}' added (id: {}, path: {})",
                name, storage.id, path
            ));
        }
        StorageCommand::Test { id } => {
            let uuid = uuid::Uuid::parse_str(id)
                .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))?;

            let storage = storage_repo
                .find_by_id(uuid)
                .await
                .map_err(|e| AppError::internal(format!("Failed to find storage: {}", e)))?
                .ok_or_else(|| AppError::not_found("Storage not found"))?;

            println!("Testing storage '{}'...", storage.name);

            match storage.provider_type {
                filehub_entity::storage::provider::StorageProviderType::Local => {
                    let root = storage
                        .config
                        .get("root_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("./data/storage/local");

                    if std::path::Path::new(root).exists() {
                        output::print_success(&format!("Local storage OK: path '{}' exists", root));
                    } else {
                        output::print_error(&format!(
                            "Local storage FAIL: path '{}' not found",
                            root
                        ));
                    }
                }
                _ => {
                    output::print_warning("Connection test not implemented for this provider type");
                }
            }
        }
    }

    Ok(())
}

/// Format bytes into human-readable string
fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
