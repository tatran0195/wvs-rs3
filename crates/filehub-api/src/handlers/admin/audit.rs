//! Audit log handlers.

use axum::Json;
use axum::extract::{Query, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::extractors::{AuthUser, PaginationParams};
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/audit
pub async fn search_audit(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
    Query(filters): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;

    let actor_id = filters
        .get("actor_id")
        .and_then(|s| Uuid::parse_str(s).ok());
    let action = filters.get("action").map(|s| s.as_str());
    let target_type = filters.get("target_type").map(|s| s.as_str());
    let target_id = filters
        .get("target_id")
        .and_then(|s| Uuid::parse_str(s).ok());

    let result = state
        .audit_service
        .search(
            &auth,
            actor_id,
            action,
            target_type,
            target_id,
            params.into_page_request(),
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}

/// GET /api/admin/audit/export
pub async fn export_audit(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Export started" } }),
    ))
}
