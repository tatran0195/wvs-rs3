//! License pool handlers.

use axum::Json;
use axum::extract::State;

use filehub_core::error::AppError;

use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/license/pool
pub async fn pool_status(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "status": "not_configured" } }),
    ))
}

/// GET /api/admin/license/pool/history
pub async fn pool_history(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(serde_json::json!({ "success": true, "data": [] })))
}

/// POST /api/admin/license/pool/reconcile
pub async fn pool_reconcile(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Reconciliation triggered" } }),
    ))
}
