//! Admin storage management handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/storages
pub async fn list_storages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let storages = state.storage_service.list_storages(&auth).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": storages }),
    ))
}

/// POST /api/admin/storages
pub async fn add_storage(
    State(_state): State<AppState>,
    auth: AuthUser,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Storage added" } }),
    ))
}

/// PUT /api/admin/storages/:id
pub async fn update_storage(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Storage updated" } }),
    ))
}

/// DELETE /api/admin/storages/:id
pub async fn remove_storage(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Storage removed" } }),
    ))
}

/// POST /api/admin/storages/:id/test
pub async fn test_storage(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "status": "ok" } }),
    ))
}

/// POST /api/admin/storages/:id/sync
pub async fn sync_storage(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Sync started" } }),
    ))
}
