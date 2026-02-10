//! Share CRUD and public access handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use uuid::Uuid;

use filehub_core::error::AppError;

use crate::dto::request::{CreateShareRequest, ShareVerifyRequest, UpdateShareRequest};
use crate::extractors::{AuthUser, PaginationParams};
use crate::state::AppState;

/// GET /api/shares
pub async fn list_shares(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .share_service
        .list_shares(&auth, params.into_page_request())
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}

/// POST /api/shares
pub async fn create_share(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateShareRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let share_type = parse_share_type(&req.share_type)?;
    let resource_type = parse_resource_type(&req.resource_type)?;
    let permission = parse_acl_permission(&req.permission)?;

    let share = state
        .share_service
        .create_share(
            &auth,
            filehub_service::share::service::CreateShareRequest {
                share_type,
                resource_type,
                resource_id: req.resource_id,
                password: req.password,
                shared_with: req.shared_with,
                permission,
                allow_download: req.allow_download,
                max_downloads: req.max_downloads,
                expires_at: req.expires_at,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": share })))
}

/// GET /api/shares/:id
pub async fn get_share(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let share = state.share_service.get_share(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": share })))
}

/// PUT /api/shares/:id
pub async fn update_share(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateShareRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let permission = req
        .permission
        .as_deref()
        .map(parse_acl_permission)
        .transpose()?;

    let share = state
        .share_service
        .update_share(
            &auth,
            id,
            filehub_service::share::service::UpdateShareRequest {
                permission,
                allow_download: req.allow_download,
                max_downloads: req.max_downloads,
                expires_at: req.expires_at,
                is_active: req.is_active,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": share })))
}

/// DELETE /api/shares/:id
pub async fn revoke_share(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.share_service.revoke_share(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Share revoked" } }),
    ))
}

/// GET /api/s/:token — public share access
pub async fn access_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let share = state.access_service.validate_token(&token).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": share })))
}

/// POST /api/s/:token/verify — verify share password
pub async fn verify_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<ShareVerifyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let share = state
        .access_service
        .verify_password(&token, &req.password)
        .await?;
    Ok(Json(serde_json::json!({ "success": true, "data": share })))
}

fn parse_share_type(s: &str) -> Result<filehub_entity::share::ShareType, AppError> {
    match s {
        "public_link" => Ok(filehub_entity::share::ShareType::PublicLink),
        "private_link" => Ok(filehub_entity::share::ShareType::PrivateLink),
        "user_share" => Ok(filehub_entity::share::ShareType::UserShare),
        _ => Err(AppError::validation(format!("Invalid share type: {s}"))),
    }
}

fn parse_resource_type(s: &str) -> Result<filehub_entity::permission::ResourceType, AppError> {
    match s {
        "file" => Ok(filehub_entity::permission::ResourceType::File),
        "folder" => Ok(filehub_entity::permission::ResourceType::Folder),
        "storage" => Ok(filehub_entity::permission::ResourceType::Storage),
        _ => Err(AppError::validation(format!("Invalid resource type: {s}"))),
    }
}

fn parse_acl_permission(s: &str) -> Result<filehub_entity::permission::AclPermission, AppError> {
    match s {
        "owner" => Ok(filehub_entity::permission::AclPermission::Owner),
        "editor" => Ok(filehub_entity::permission::AclPermission::Editor),
        "commenter" => Ok(filehub_entity::permission::AclPermission::Commenter),
        "viewer" => Ok(filehub_entity::permission::AclPermission::Viewer),
        _ => Err(AppError::validation(format!("Invalid permission: {s}"))),
    }
}
