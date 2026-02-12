//! Storage listing and usage handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::extractors::AuthUser;
use crate::state::AppState;

/// GET /api/storages
pub async fn list_storages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let storages = state.storage_service.list_storages(&auth).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": storages }),
    ))
}

/// GET /api/storages/:id
pub async fn get_storage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let storage = state.storage_service.get_storage(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": storage }),
    ))
}

/// GET /api/storages/:id/usage
pub async fn get_usage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let usage = state.storage_service.get_usage(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": usage })))
}

/// POST /api/storages/:id/transfer
pub async fn initiate_transfer(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // let transfer = state
    //     .storage_service
    //     .initiate_transfer(&auth, id, req)
    //     .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": "" })))
}
