//! Notification handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::dto::request::UpdatePreferencesRequest;
use crate::dto::response::{ApiResponse, CountResponse};
use crate::extractors::{AuthUser, PaginationParams};
use crate::state::AppState;
use axum::extract::Query;

/// GET /api/notifications
pub async fn list_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .notification_service
        .list_notifications(&auth, params.into_page_request())
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}

/// GET /api/notifications/unread-count
pub async fn unread_count(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let count = state.notification_service.unread_count(&auth).await?;
    Ok(Json(ApiResponse::ok(CountResponse { count })))
}

/// PUT /api/notifications/:id/read
pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.notification_service.mark_read(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Marked as read" } }),
    ))
}

/// PUT /api/notifications/read-all
pub async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state.notification_service.mark_all_read(&auth).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "marked": count } }),
    ))
}

/// DELETE /api/notifications/:id
pub async fn dismiss(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.notification_service.dismiss(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Dismissed" } }),
    ))
}

/// GET /api/notifications/preferences
pub async fn get_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let prefs = state.notification_service.get_preferences(&auth).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": prefs })))
}

/// PUT /api/notifications/preferences
pub async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let prefs = state
        .notification_service
        .update_preferences(&auth, req.preferences)
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": prefs })))
}
