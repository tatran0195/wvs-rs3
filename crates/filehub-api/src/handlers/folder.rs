//! Folder CRUD and tree handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_service::folder::service::{
    CreateFolderRequest as SvcCreateFolder, MoveFolderRequest as SvcMoveFolder,
};

use crate::dto::request::CreateFolderRequest;
use crate::extractors::AuthUser;
use crate::state::AppState;

/// GET /api/folders?storage_id=...
pub async fn list_root_folders(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let storage_id = params
        .get("storage_id")
        .ok_or_else(|| AppError::validation("storage_id is required"))?
        .parse::<Uuid>()
        .map_err(|_| AppError::validation("Invalid storage_id"))?;

    let folders = state
        .folder_service
        .list_root_folders(&auth, storage_id)
        .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": folders }),
    ))
}

/// GET /api/folders/:id
pub async fn get_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder = state.folder_service.get_folder(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": folder })))
}

/// GET /api/folders/:id/children
pub async fn list_children(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let children = state.folder_service.list_children(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": children }),
    ))
}

/// GET /api/folders/:id/tree
pub async fn get_tree(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tree = state.tree_service.get_tree(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": tree })))
}

/// POST /api/folders
pub async fn create_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateFolderRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder = state
        .folder_service
        .create_folder(
            &auth,
            SvcCreateFolder {
                storage_id: req.storage_id,
                parent_id: req.parent_id,
                name: req.name,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": folder })))
}

/// PUT /api/folders/:id
pub async fn update_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let name = req
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("name is required"))?;

    let folder = state.folder_service.update_folder(&auth, id, name).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": folder })))
}

/// PUT /api/folders/:id/move
pub async fn move_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_parent_id = req
        .get("new_parent_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::validation("new_parent_id is required"))?;

    let folder = state
        .folder_service
        .move_folder(&auth, id, SvcMoveFolder { new_parent_id })
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": folder })))
}

/// DELETE /api/folders/:id
pub async fn delete_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.folder_service.delete_folder(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Folder deleted" } }),
    ))
}
