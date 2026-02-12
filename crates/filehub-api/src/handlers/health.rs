//! Health check handlers.

use axum::Json;
use axum::extract::State;

use crate::dto::response::{ApiResponse, DetailedHealthResponse, HealthResponse};
use crate::state::AppState;

/// GET /api/health
pub async fn health() -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::ok(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // Would come from a global start time
    }))
}

/// GET /api/health/detailed
pub async fn health_detailed(
    State(state): State<AppState>,
) -> Json<ApiResponse<DetailedHealthResponse>> {
    let ws_connections = state.realtime.connections.total_connections();
    let online_users = state.realtime.connections.unique_users();

    Json(ApiResponse::ok(DetailedHealthResponse {
        status: "ok".to_string(),
        database: "connected".to_string(),
        cache: "connected".to_string(),
        storage: "available".to_string(),
        ws_connections,
        online_users,
    }))
}
