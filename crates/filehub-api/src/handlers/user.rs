//! User self-service handlers.

use axum::Json;
use axum::extract::State;

use filehub_core::error::AppError;
use filehub_service::user::service::UpdateProfileRequest as SvcUpdateProfile;

use crate::dto::request::{ChangePasswordRequest, UpdateProfileRequest};
use crate::dto::response::{ApiResponse, MessageResponse, UserResponse};
use crate::extractors::AuthUser;
use crate::state::AppState;

/// GET /api/users/me
pub async fn get_profile(
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

/// PUT /api/users/me
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let user = state
        .user_service
        .update_profile(
            &auth,
            SvcUpdateProfile {
                display_name: req.display_name,
                email: req.email,
            },
        )
        .await?;

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

/// PUT /api/users/me/password
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, AppError> {
    state
        .user_service
        .change_password(&auth, &req.current_password, &req.new_password)
        .await?;

    Ok(Json(ApiResponse::ok(MessageResponse {
        message: "Password changed successfully".to_string(),
    })))
}
