//! Report handlers.

use axum::Json;
use axum::extract::State;

use filehub_core::error::AppError;

use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/reports/weekly
pub async fn weekly_report(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let report = state.report_service.generate().await?;
    Ok(Json(serde_json::json!({ "success": true, "data": report })))
}

/// GET /api/admin/reports/storage-usage
pub async fn storage_usage(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(serde_json::json!({ "success": true, "data": [] })))
}
