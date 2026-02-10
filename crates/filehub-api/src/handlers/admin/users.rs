//! Admin user management handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_entity::user::{UserRole, UserStatus};
use filehub_service::user::admin::{AdminUpdateUserRequest, CreateUserRequest as SvcCreateUser};

use crate::dto::request::{
    ChangeRoleRequest, ChangeStatusRequest, CreateUserRequest, ResetPasswordRequest,
};
use crate::extractors::{AuthUser, PaginationParams};
use crate::middleware::rbac::require_admin;
use crate::state::AppState;

/// GET /api/admin/users
pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let result = state
        .admin_user_service
        .list_users(&auth, params.into_page_request())
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}

/// POST /api/admin/users
pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let role = parse_role(&req.role)?;
    let user = state
        .admin_user_service
        .create_user(
            &auth,
            SvcCreateUser {
                username: req.username,
                email: req.email,
                password: req.password,
                display_name: req.display_name,
                role,
            },
        )
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": user })))
}

/// GET /api/admin/users/:id
pub async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let user = state.admin_user_service.get_user(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": user })))
}

/// PUT /api/admin/users/:id
pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let user = state
        .admin_user_service
        .update_user(
            &auth,
            id,
            AdminUpdateUserRequest {
                display_name: req
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                email: req.get("email").and_then(|v| v.as_str()).map(String::from),
            },
        )
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": user })))
}

/// PUT /api/admin/users/:id/role
pub async fn change_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let role = parse_role(&req.role)?;
    let user = state
        .admin_user_service
        .change_role(&auth, id, role)
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": user })))
}

/// PUT /api/admin/users/:id/status
pub async fn change_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    let status = parse_status(&req.status)?;
    let user = state
        .admin_user_service
        .change_status(&auth, id, status)
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": user })))
}

/// PUT /api/admin/users/:id/reset-password
pub async fn reset_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    state
        .admin_user_service
        .reset_password(&auth, id, &req.new_password)
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Password reset" } }),
    ))
}

/// DELETE /api/admin/users/:id
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth)?;
    state.admin_user_service.delete_user(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "User deleted" } }),
    ))
}

fn parse_role(s: &str) -> Result<UserRole, AppError> {
    match s {
        "admin" => Ok(UserRole::Admin),
        "manager" => Ok(UserRole::Manager),
        "creator" => Ok(UserRole::Creator),
        "viewer" => Ok(UserRole::Viewer),
        _ => Err(AppError::validation(format!("Invalid role: {s}"))),
    }
}

fn parse_status(s: &str) -> Result<UserStatus, AppError> {
    match s {
        "active" => Ok(UserStatus::Active),
        "inactive" => Ok(UserStatus::Inactive),
        "locked" => Ok(UserStatus::Locked),
        _ => Err(AppError::validation(format!("Invalid status: {s}"))),
    }
}
