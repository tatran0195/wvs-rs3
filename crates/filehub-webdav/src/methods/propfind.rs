//! PROPFIND method implementation (RFC 4918 Section 9.1).

use std::sync::Arc;

use http::{Response, StatusCode};
use tracing;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::id::{FolderId, StorageId};
use filehub_service::file::service::FileService;
use filehub_service::folder::service::FolderService;

use crate::auth::DavUser;
use crate::properties::{DavResource, Depth, PropfindRequest, build_multistatus_xml};

/// Handle a PROPFIND request
pub async fn handle_propfind(
    user: &DavUser,
    storage_id: StorageId,
    path: &str,
    depth: Depth,
    body: &str,
    folder_service: &Arc<FolderService>,
    file_service: &Arc<FileService>,
    base_href: &str,
) -> Result<Response<String>, AppError> {
    let _propfind_req = PropfindRequest::parse(body);

    tracing::debug!(
        "PROPFIND: user={}, storage={}, path='{}', depth={:?}",
        user.username,
        storage_id,
        path,
        depth
    );

    let normalized_path = normalize_path(path);
    let mut resources = Vec::new();

    if normalized_path == "/" || normalized_path.is_empty() {
        let root_resource = DavResource::collection(
            format!("{}/", base_href),
            "".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        resources.push(root_resource);

        if depth != Depth::Zero {
            let folders = folder_service
                .list_root_folders(storage_id, user.id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to list root folders: {}", e)))?;

            for folder in &folders {
                let href = format!("{}/{}/", base_href, percent_encode(&folder.name));
                resources.push(DavResource::collection(
                    href,
                    folder.name.clone(),
                    folder.updated_at,
                    folder.created_at,
                ));
            }

            let files = file_service
                .list_root_files(storage_id, user.id)
                .await
                .unwrap_or_default();

            for file in &files {
                let href = format!("{}/{}", base_href, percent_encode(&file.name));
                resources.push(DavResource::file(
                    href,
                    file.name.clone(),
                    file.size_bytes as u64,
                    file.mime_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                    file.updated_at,
                    file.created_at,
                    file.checksum_sha256.clone(),
                ));
            }
        }
    } else {
        let folder = folder_service
            .find_by_path(storage_id, &normalized_path, user.id)
            .await;

        match folder {
            Ok(folder) => {
                let folder_href = format!("{}{}/", base_href, path_to_href(&normalized_path));
                resources.push(DavResource::collection(
                    folder_href,
                    folder.name.clone(),
                    folder.updated_at,
                    folder.created_at,
                ));

                if depth != Depth::Zero {
                    let children = folder_service
                        .list_children(FolderId::from(folder.id), storage_id, user.id)
                        .await
                        .unwrap_or_default();

                    for child in &children {
                        let child_href = format!(
                            "{}{}/{}/",
                            base_href,
                            path_to_href(&normalized_path),
                            percent_encode(&child.name)
                        );
                        resources.push(DavResource::collection(
                            child_href,
                            child.name.clone(),
                            child.updated_at,
                            child.created_at,
                        ));
                    }

                    let files = file_service
                        .list_by_folder(FolderId::from(folder.id), user.id)
                        .await
                        .unwrap_or_default();

                    for file in &files {
                        let file_href = format!(
                            "{}{}/{}",
                            base_href,
                            path_to_href(&normalized_path),
                            percent_encode(&file.name)
                        );
                        resources.push(DavResource::file(
                            file_href,
                            file.name.clone(),
                            file.size_bytes as u64,
                            file.mime_type
                                .clone()
                                .unwrap_or_else(|| "application/octet-stream".to_string()),
                            file.updated_at,
                            file.created_at,
                            file.checksum_sha256.clone(),
                        ));
                    }
                }
            }
            Err(_) => {
                let file = file_service
                    .find_by_path(storage_id, &normalized_path, user.id)
                    .await
                    .map_err(|_| AppError::not_found("Resource not found"))?;

                let file_href = format!("{}{}", base_href, path_to_href(&normalized_path));
                resources.push(DavResource::file(
                    file_href,
                    file.name.clone(),
                    file.size_bytes as u64,
                    file.mime_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                    file.updated_at,
                    file.created_at,
                    file.checksum_sha256.clone(),
                ));
            }
        }
    }

    let xml = build_multistatus_xml(&resources);

    let response = Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(xml)
        .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Normalize a path by removing leading/trailing slashes and double slashes
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", trimmed)
    }
}

/// Convert a normalized path to href segments
fn path_to_href(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| percent_encode(s))
        .collect::<Vec<_>>()
        .join("/")
        .to_string()
}

/// Percent-encode a path segment
fn percent_encode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
