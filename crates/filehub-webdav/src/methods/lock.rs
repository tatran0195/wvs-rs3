//! LOCK and UNLOCK method implementations (RFC 4918 Section 9.10/9.11).
//!
//! FileHub provides basic locking support. Locks are advisory and mapped
//! to the file locking feature in the file service.

use std::sync::Arc;

use http::{Response, StatusCode};
use tracing;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::id::{FileId, StorageId};
use filehub_service::file::service::FileService;

use crate::auth::DavUser;

/// Handle a LOCK request
pub async fn handle_lock(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    _body: &str,
    file_service: &Arc<FileService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!("LOCK: user={}, path='{}'", user.username, path);

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("Cannot lock root collection".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let file = file_service
        .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
        .await
        .map_err(|_| AppError::not_found("File not found"))?;

    file_service
        .lock_file(FileId::from(file.id), user.id)
        .await
        .map_err(|e| {
            if e.to_string().contains("already locked") {
                AppError::conflict("Resource is already locked")
            } else {
                AppError::internal(format!("Failed to lock file: {}", e))
            }
        })?;

    let lock_token = format!("opaquelocktoken:{}", Uuid::new_v4());

    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:prop xmlns:D="DAV:">
  <D:lockdiscovery>
    <D:activelock>
      <D:locktype><D:write/></D:locktype>
      <D:lockscope><D:exclusive/></D:lockscope>
      <D:depth>0</D:depth>
      <D:owner>{}</D:owner>
      <D:timeout>Second-3600</D:timeout>
      <D:locktoken>
        <D:href>{}</D:href>
      </D:locktoken>
    </D:activelock>
  </D:lockdiscovery>
</D:prop>"#,
        user.username, lock_token
    );

    tracing::info!(
        "Locked file via WebDAV: path='{}', token='{}'",
        path,
        lock_token
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Lock-Token", format!("<{}>", lock_token))
        .body(xml)
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
}

/// Handle an UNLOCK request
pub async fn handle_unlock(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    _lock_token: Option<&str>,
    file_service: &Arc<FileService>,
) -> Result<Response<String>, AppError> {
    tracing::debug!("UNLOCK: user={}, path='{}'", user.username, path);

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("Cannot unlock root collection".to_string())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
    }

    let file = file_service
        .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
        .await
        .map_err(|_| AppError::not_found("File not found"))?;

    file_service
        .unlock_file(FileId::from(file.id), user.id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to unlock file: {}", e)))?;

    tracing::info!("Unlocked file via WebDAV: path='{}'", path);

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(String::new())
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
}
