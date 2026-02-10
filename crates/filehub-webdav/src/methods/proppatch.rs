//! PROPPATCH method implementation (RFC 4918 Section 9.2).

use http::{Response, StatusCode};
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::StorageId;

use crate::auth::DavUser;
use crate::properties::build_status_xml;

/// Handle a PROPPATCH request
///
/// FileHub treats most properties as read-only. PROPPATCH requests
/// are accepted but property changes are not persisted (returns 200 OK).
pub async fn handle_proppatch(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    _body: &str,
    base_href: &str,
) -> Result<Response<String>, AppError> {
    tracing::debug!(
        "PROPPATCH: user={}, path='{}' (properties are read-only)",
        user.username,
        path
    );

    let href = format!("{}{}", base_href, path);
    let xml = build_status_xml(&href, 200, "OK");

    let response = Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(xml)
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}
