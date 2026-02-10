//! Admin session management handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::dto::request::{
    BulkTerminateRequest, SendSessionMessageRequest, TerminateSessionRequest,
};
use crate::extractors::AuthUser;
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/sessions
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let sessions = state.session_service.list_active_sessions(&auth).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": sessions }),
    ))
}

/// GET /api/admin/sessions/:id
pub async fn get_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let session = state.session_service.get_session(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": session }),
    ))
}

/// POST /api/admin/sessions/:id/terminate
pub async fn terminate_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<TerminateSessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    state
        .termination_service
        .terminate_session(&auth, id, &req.reason)
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Session terminated" } }),
    ))
}

/// POST /api/admin/sessions/terminate-bulk
pub async fn terminate_bulk(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BulkTerminateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let count = state
        .termination_service
        .bulk_terminate(
            &auth,
            filehub_service::session::termination::BulkTerminateRequest {
                session_ids: req.session_ids,
                reason: req.reason,
            },
        )
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "terminated": count } }),
    ))
}

/// POST /api/admin/sessions/terminate-all
pub async fn terminate_all(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TerminateSessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let count = state
        .termination_service
        .terminate_all_non_admin(&auth, &req.reason)
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "terminated": count } }),
    ))
}

/// POST /api/admin/sessions/:id/send-message
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SendSessionMessageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;

    let msg = filehub_realtime::message::types::OutboundMessage::AdminBroadcast {
        id: Uuid::new_v4(),
        title: "Admin Message".to_string(),
        message: req.message,
        severity: "info".to_string(),
        persistent: false,
        action: None,
        timestamp: chrono::Utc::now(),
    };

    state.realtime.connections.send_to_session(&id, &msg).await;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Message sent" } }),
    ))
}
