//! GET, HEAD, and PUT method implementations for WebDAV.

use std::sync::Arc;

use bytes::Bytes;
use http::{Response, StatusCode};
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::{FileId, FolderId, StorageId};
use filehub_service::file::service::FileService;
use filehub_service::file::upload::UploadService;
use filehub_service::folder::service::FolderService;

use crate::auth::DavUser;
use crate::properties::format_http_date;

/// Handle a GET request (download file)
pub async fn handle_get(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    file_service: &Arc<FileService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!("GET: user={}, path='{}'", user.username, path);

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("Cannot GET a collection".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let file = file_service
        .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
        .await
        .map_err(|_| AppError::not_found("File not found"))?;

    let content = file_service
        .read_file_content(FileId::from(file.id), user.id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;

    let content_str = match String::from_utf8(content.clone()) {
        Ok(s) => s,
        Err(_) => {
            let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content);
            b64
        }
    };

    let mime = file
        .mime_type
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &mime)
        .header("Content-Length", file.size_bytes.to_string())
        .header("Last-Modified", format_http_date(&file.updated_at));

    if let Some(ref etag) = file.checksum_sha256 {
        builder = builder.header("ETag", format!("\"{}\"", etag));
    }

    builder
        .body(content_str)
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))
}

/// Handle a HEAD request (file metadata only)
pub async fn handle_head(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    file_service: &Arc<FileService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!("HEAD: user={}, path='{}'", user.username, path);

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "httpd/unix-directory")
            .body(String::new())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let file = file_service
        .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
        .await
        .map_err(|_| AppError::not_found("File not found"))?;

    let mime = file
        .mime_type
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &mime)
        .header("Content-Length", file.size_bytes.to_string())
        .header("Last-Modified", format_http_date(&file.updated_at));

    if let Some(ref etag) = file.checksum_sha256 {
        builder = builder.header("ETag", format!("\"{}\"", etag));
    }

    builder
        .body(String::new())
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))
}

/// Handle a PUT request (upload/overwrite file)
pub async fn handle_put(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    body: Bytes,
    content_type: Option<&str>,
    file_service: &Arc<FileService>,
    upload_service: &Arc<UploadService>,
    folder_service: &Arc<FolderService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!(
        "PUT: user={}, path='{}', size={}",
        user.username,
        path,
        body.len()
    );

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::CONFLICT)
            .body("Cannot PUT to root".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    let file_name = parts
        .last()
        .ok_or_else(|| AppError::bad_request("Invalid path"))?
        .to_string();

    let parent_folder_id = if parts.len() > 1 {
        let parent_path = format!("/{}", parts[..parts.len() - 1].join("/"));
        let parent = folder_service
            .find_by_path(storage_id, &parent_path, user.id)
            .await
            .map_err(|_| AppError::conflict("Parent folder does not exist"))?;
        FolderId::from(parent.id)
    } else {
        let root = folder_service
            .get_or_create_root(storage_id, user.id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get root folder: {}", e)))?;
        FolderId::from(root.id)
    };

    let mime = content_type.map(|s| s.to_string()).unwrap_or_else(|| {
        mime_guess::from_path(&file_name)
            .first_or_octet_stream()
            .to_string()
    });

    let existing = file_service
        .find_by_name_in_folder(parent_folder_id, &file_name, user.id)
        .await;

    let status = match existing {
        Ok(existing_file) => {
            upload_service
                .overwrite_file(
                    FileId::from(existing_file.id),
                    body.to_vec(),
                    Some(mime),
                    user.id,
                )
                .await
                .map_err(|e| AppError::internal(format!("Failed to overwrite file: {}", e)))?;

            tracing::info!("Overwrote file via WebDAV: path='{}'", path);
            StatusCode::NO_CONTENT
        }
        Err(_) => {
            upload_service
                .simple_upload(
                    storage_id,
                    parent_folder_id,
                    &file_name,
                    body.to_vec(),
                    Some(mime),
                    user.id,
                )
                .await
                .map_err(|e| AppError::internal(format!("Failed to upload file: {}", e)))?;

            tracing::info!("Created file via WebDAV: path='{}'", path);
            StatusCode::CREATED
        }
    };

    Ok(Response::builder()
        .status(status)
        .body(String::new())
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
}
