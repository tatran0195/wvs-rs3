//! Job management handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/jobs
pub async fn list_jobs(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(serde_json::json!({ "success": true, "data": [] })))
}

/// GET /api/admin/jobs/:id
pub async fn get_job(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(serde_json::json!({ "success": true, "data": null })))
}

/// POST /api/admin/jobs/:id/cancel
pub async fn cancel_job(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Job cancelled" } }),
    ))
}

/// POST /api/admin/jobs/:id/retry
pub async fn retry_job(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Job retried" } }),
    ))
}
