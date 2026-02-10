//! MKCOL method implementation (RFC 4918 Section 9.3).

use std::sync::Arc;

use http::{Response, StatusCode};
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::StorageId;
use filehub_service::folder::service::FolderService;

use crate::auth::DavUser;

/// Handle a MKCOL request (create collection/folder)
pub async fn handle_mkcol(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    body: &str,
    folder_service: &Arc<FolderService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!("MKCOL: user={}, path='{}'", user.username, path);

    if !body.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
            .body("MKCOL with body is not supported".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("Cannot create root collection".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    let folder_name = parts
        .last()
        .ok_or_else(|| AppError::bad_request("Invalid path"))?
        .to_string();

    let parent_path = if parts.len() > 1 {
        Some(format!("/{}", parts[..parts.len() - 1].join("/")))
    } else {
        None
    };

    let parent_id = if let Some(ref pp) = parent_path {
        let parent = folder_service
            .find_by_path(storage_id, pp, user.id)
            .await
            .map_err(|_| AppError::conflict("Parent folder does not exist"))?;
        Some(parent.id)
    } else {
        None
    };

    folder_service
        .create_folder(
            storage_id,
            parent_id.map(|id| filehub_core::types::id::FolderId::from(id)),
            &folder_name,
            user.id,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("already exists") {
                AppError::conflict("Collection already exists")
            } else {
                e
            }
        })?;

    tracing::info!("Created collection: path='{}'", path);

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .body(String::new())
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
}
