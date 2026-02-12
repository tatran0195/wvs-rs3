//! Folder management CLI commands.

use clap::{Args, Subcommand};
use filehub_entity::folder::CreateFolder;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, OutputFormat};
use filehub_core::error::AppError;
use filehub_database::repositories::folder::FolderRepository;

/// Arguments for folder commands
#[derive(Debug, Args)]
pub struct FolderArgs {
    /// Folder subcommand
    #[command(subcommand)]
    pub command: FolderCommand,
}

/// Folder subcommands
#[derive(Debug, Subcommand)]
pub enum FolderCommand {
    /// List root folders for a storage
    List {
        /// Storage ID
        #[arg(short, long)]
        storage_id: String,
    },
    /// Create a new folder
    Create {
        /// Storage ID
        #[arg(short, long)]
        storage_id: String,
        /// Folder name
        #[arg(short, long)]
        name: String,
        /// Parent folder ID (omit for root)
        #[arg(short, long)]
        parent_id: Option<String>,
    },
    /// Show folder tree
    Tree {
        /// Storage ID
        #[arg(short, long)]
        storage_id: String,
        /// Max depth
        #[arg(short, long, default_value = "3")]
        depth: u32,
    },
}

/// Folder display row
#[derive(Debug, Serialize, Tabled)]
struct FolderRow {
    /// Folder ID
    id: String,
    /// Name
    name: String,
    /// Path
    path: String,
    /// Depth
    depth: i32,
    /// Created at
    created_at: String,
}

/// Execute folder commands
pub async fn execute(
    args: &FolderArgs,
    config_path: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;
    let folder_repo = FolderRepository::new(pool.clone());

    match &args.command {
        FolderCommand::List { storage_id } => {
            let sid = uuid::Uuid::parse_str(storage_id)
                .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))?;

            let folders = folder_repo
                .find_roots(sid)
                .await
                .map_err(|e| AppError::internal(format!("Failed to list folders: {}", e)))?;

            let rows: Vec<FolderRow> = folders
                .iter()
                .map(|f| FolderRow {
                    id: f.id.to_string(),
                    name: f.name.clone(),
                    path: f.path.clone(),
                    depth: f.depth,
                    created_at: f.created_at.format("%Y-%m-%d %H:%M").to_string(),
                })
                .collect();

            output::print_list(&rows, format);
        }
        FolderCommand::Create {
            storage_id,
            name,
            parent_id,
        } => {
            let sid = uuid::Uuid::parse_str(storage_id)
                .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))?;

            let pid = parent_id
                .as_ref()
                .map(|p| {
                    uuid::Uuid::parse_str(p)
                        .map_err(|e| AppError::bad_request(&format!("Invalid parent UUID: {}", e)))
                })
                .transpose()?;

            let (path, depth) = if let Some(pid) = pid {
                let parent = folder_repo
                    .find_by_id(pid)
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to find parent: {}", e)))?
                    .ok_or_else(|| AppError::not_found("Parent folder not found"))?;
                (format!("{}/{}", parent.path, name), parent.depth + 1)
            } else {
                (format!("/{}", name), 0)
            };

            let folder = CreateFolder {
                storage_id: sid,
                parent_id: pid,
                name: name.clone(),
                path,
                depth,
                owner_id: uuid::Uuid::nil(),
            };

            let folder = folder_repo
                .create(&folder)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create folder: {}", e)))?;

            output::print_success(&format!("Folder '{}' created (id: {})", name, folder.id));
        }
        FolderCommand::Tree { storage_id, depth } => {
            let sid = uuid::Uuid::parse_str(storage_id)
                .map_err(|e| AppError::bad_request(&format!("Invalid UUID: {}", e)))?;

            let folders = folder_repo
                .find_roots(sid)
                .await
                .map_err(|e| AppError::internal(format!("Failed to get folder tree: {}", e)))?;

            println!("/");
            for folder in &folders {
                if folder.depth <= *depth as i32 {
                    let indent = "  ".repeat(folder.depth as usize + 1);
                    println!("{}├── {}/", indent, folder.name);
                }
            }
        }
    }

    Ok(())
}
