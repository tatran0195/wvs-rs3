//! Admin broadcast handlers.

use axum::Json;
use axum::extract::State;

use filehub_core::error::AppError;

use crate::dto::request::BroadcastRequest;
use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// POST /api/admin/broadcast
pub async fn send_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BroadcastRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;

    state
        .realtime
        .notifications
        .broadcast_admin_message(
            uuid::Uuid::new_v4(),
            &req.title,
            &req.message,
            &req.severity,
            req.persistent,
        )
        .await;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Broadcast sent" } }),
    ))
}

/// GET /api/admin/broadcast/history
pub async fn broadcast_history(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    Ok(Json(serde_json::json!({ "success": true, "data": [] })))
}
