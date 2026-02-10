//! Auth handlers — login, logout, refresh, me.

use axum::Json;
use axum::extract::State;
use std::net::IpAddr;

use filehub_core::error::AppError;

use crate::dto::request::{LoginRequest, RefreshRequest};
use crate::dto::response::{ApiResponse, LoginResponse, UserResponse};
use crate::extractors::AuthUser;
use crate::state::AppState;

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let ip: IpAddr = "127.0.0.1"
        .parse()
        .unwrap_or_else(|_| "0.0.0.0".parse().unwrap());

    let result = state
        .session_manager
        .login(&req.username, &req.password, ip, None, None)
        .await?;

    let user_resp = UserResponse {
        id: result.user.id,
        username: result.user.username.clone(),
        email: result.user.email.clone(),
        display_name: result.user.display_name.clone(),
        role: result.user.role.to_string(),
        status: result.user.status.to_string(),
        created_at: result.user.created_at,
        last_login_at: result.user.last_login_at,
    };

    Ok(Json(ApiResponse::ok(LoginResponse {
        access_token: result.tokens.access_token,
        refresh_token: result.tokens.refresh_token,
        access_expires_at: result.tokens.access_expires_at,
        refresh_expires_at: result.tokens.refresh_expires_at,
        user: user_resp,
    })))
}

/// POST /api/auth/logout
pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<crate::dto::response::MessageResponse>>, AppError> {
    // We need the claims; reconstruct from context
    // In practice, you'd pass the raw token. For now, we use session_manager directly.
    // The session manager's logout requires Claims, which we already validated.
    // We'll use admin_terminate with self as admin for simplicity.
    state
        .session_manager
        .admin_terminate(auth.session_id, auth.user_id, "User logout")
        .await?;

    Ok(Json(ApiResponse::ok(
        crate::dto::response::MessageResponse {
            message: "Logged out successfully".to_string(),
        },
    )))
}

/// POST /api/auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let tokens = state.session_manager.refresh(&req.refresh_token).await?;

    // We don't return user info on refresh; partial response
    Ok(Json(ApiResponse::ok(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        access_expires_at: tokens.access_expires_at,
        refresh_expires_at: tokens.refresh_expires_at,
        user: UserResponse {
            id: uuid::Uuid::nil(),
            username: String::new(),
            email: None,
            display_name: None,
            role: String::new(),
            status: String::new(),
            created_at: chrono::Utc::now(),
            last_login_at: None,
        },
    })))
}

/// GET /api/auth/me
pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let user = state.user_service.get_profile(&auth).await?;

    Ok(Json(ApiResponse::ok(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        role: user.role.to_string(),
        status: user.status.to_string(),
        created_at: user.created_at,
        last_login_at: user.last_login_at,
    })))
}
