//! MOVE method implementation (RFC 4918 Section 9.9).

use std::sync::Arc;

use http::{HeaderMap, Response, StatusCode};
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::StorageId;
use filehub_service::file::service::FileService;
use filehub_service::folder::service::FolderService;

use crate::auth::DavUser;

/// Handle a MOVE request
pub async fn handle_move(
    user: &DavUser,
    storage_id: StorageId,
    source_path: &str,
    headers: &HeaderMap,
    file_service: &Arc<FileService>,
    folder_service: &Arc<FolderService>,
    base_href: &str,
) -> Result<Response<String>, AppError> {
    let destination = headers
        .get("Destination")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("Missing Destination header"))?;

    let overwrite = headers
        .get("Overwrite")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("T")
        == "T";

    let dest_path = extract_path_from_destination(destination, base_href);

    tracing::debug!(
        "MOVE: user={}, src='{}', dst='{}', overwrite={}",
        user.username,
        source_path,
        dest_path,
        overwrite
    );

    let source_trimmed = source_path.trim_matches('/');
    let dest_trimmed = dest_path.trim_matches('/');

    if source_trimmed.is_empty() || dest_trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body("Cannot move root collection".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let folder_result = folder_service
        .find_by_path(storage_id, &format!("/{}", source_trimmed), user.id)
        .await;

    match folder_result {
        Ok(folder) => {
            folder_service
                .move_folder(
                    filehub_core::types::id::FolderId::from(folder.id),
                    storage_id,
                    &dest_path,
                    overwrite,
                    user.id,
                )
                .await
                .map_err(|e| AppError::internal(format!("Folder move failed: {}", e)))?;
        }
        Err(_) => {
            let file = file_service
                .find_by_path(storage_id, &format!("/{}", source_trimmed), user.id)
                .await
                .map_err(|_| AppError::not_found("Source resource not found"))?;

            file_service
                .move_file(
                    filehub_core::types::id::FileId::from(file.id),
                    storage_id,
                    &dest_path,
                    overwrite,
                    user.id,
                )
                .await
                .map_err(|e| AppError::internal(format!("File move failed: {}", e)))?;
        }
    }

    tracing::info!("MOVE completed: '{}' → '{}'", source_path, dest_path);

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .body(String::new())
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
}

/// Extract the relative path from a full Destination URI
fn extract_path_from_destination(destination: &str, base_href: &str) -> String {
    if let Some(idx) = destination.find(base_href) {
        let path = &destination[idx + base_href.len()..];
        format!("/{}", path.trim_matches('/'))
    } else {
        let uri_parts: Vec<&str> = destination.splitn(4, '/').collect();
        if uri_parts.len() >= 4 {
            format!("/{}", uri_parts[3..].join("/").trim_matches('/'))
        } else {
            destination.to_string()
        }
    }
}
