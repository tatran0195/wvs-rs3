//! File upload CLI command.

use std::path::PathBuf;

use clap::Args;

use crate::output;
use filehub_core::error::AppError;

/// Arguments for the upload command
#[derive(Debug, Args)]
pub struct UploadArgs {
    /// Path to the file to upload
    pub file: PathBuf,

    /// Target storage ID
    #[arg(short, long)]
    pub storage_id: String,

    /// Target folder ID
    #[arg(short, long)]
    pub folder_id: String,

    /// Override file name
    #[arg(short, long)]
    pub name: Option<String>,
}

/// Execute the upload command
pub async fn execute(args: &UploadArgs, config_path: &str) -> Result<(), AppError> {
    let config = super::load_config(config_path).await?;
    let pool = super::create_db_pool(&config).await?;

    if !args.file.exists() {
        return Err(AppError::not_found(&format!(
            "File not found: {}",
            args.file.display()
        )));
    }

    let file_name = args.name.clone().unwrap_or_else(|| {
        args.file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload")
            .to_string()
    });

    let content = tokio::fs::read(&args.file)
        .await
        .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;

    let size = content.len();

    println!(
        "Uploading '{}' ({} bytes) to storage={}, folder={}...",
        file_name, size, args.storage_id, args.folder_id
    );

    let storage_id = uuid::Uuid::parse_str(&args.storage_id)
        .map_err(|e| AppError::bad_request(&format!("Invalid storage UUID: {}", e)))?;
    let folder_id = uuid::Uuid::parse_str(&args.folder_id)
        .map_err(|e| AppError::bad_request(&format!("Invalid folder UUID: {}", e)))?;

    let mime = mime_guess::from_path(&file_name)
        .first_or_octet_stream()
        .to_string();

    let file_repo = filehub_database::repositories::file::FileRepository::new(pool.clone());

    let file = filehub_entity::file::model::File {
        id: uuid::Uuid::new_v4(),
        folder_id,
        storage_id,
        name: file_name.clone(),
        storage_path: format!("/{}", file_name),
        mime_type: Some(mime),
        size_bytes: size as i64,
        checksum_sha256: None,
        metadata: serde_json::json!({}),
        current_version: 1,
        is_locked: false,
        locked_by: None,
        locked_at: None,
        owner_id: uuid::Uuid::nil(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    file_repo
        .create(&file)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create file record: {}", e)))?;

    output::print_success(&format!(
        "File '{}' uploaded (id: {}, size: {} bytes)",
        file_name, file.id, size
    ));

    Ok(())
}
