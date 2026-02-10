//! ACL permission management handlers.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_entity::permission::{AclInheritance, AclPermission, ResourceType};

use crate::dto::request::CreateAclEntryRequest;
use crate::extractors::AuthUser;
use crate::state::AppState;

/// GET /api/permissions/:type/:id
pub async fn get_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((res_type, res_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rt = parse_resource_type(&res_type)?;
    let entries = state
        .permission_service
        .get_entries(&auth, rt, res_id)
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": entries }),
    ))
}

/// POST /api/permissions/:type/:id
pub async fn add_permission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((res_type, res_id)): Path<(String, Uuid)>,
    Json(req): Json<CreateAclEntryRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rt = parse_resource_type(&res_type)?;
    let permission = parse_permission(&req.permission)?;
    let inheritance = parse_inheritance(&req.inheritance)?;

    let entry = state
        .permission_service
        .add_entry(
            &auth,
            rt,
            res_id,
            filehub_service::permission::service::CreateAclEntryRequest {
                user_id: req.user_id,
                is_anyone: req.is_anyone,
                permission,
                inheritance,
                expires_at: req.expires_at,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": entry })))
}

/// PUT /api/permissions/entry/:id
pub async fn update_permission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(entry_id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let permission = req
        .get("permission")
        .and_then(|v| v.as_str())
        .map(parse_permission)
        .transpose()?;
    let inheritance = req
        .get("inheritance")
        .and_then(|v| v.as_str())
        .map(parse_inheritance)
        .transpose()?;

    let entry = state
        .permission_service
        .update_entry(
            &auth,
            entry_id,
            filehub_service::permission::service::UpdateAclEntryRequest {
                permission,
                inheritance,
                expires_at: None,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": entry })))
}

/// DELETE /api/permissions/entry/:id
pub async fn remove_permission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(entry_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .permission_service
        .remove_entry(&auth, entry_id)
        .await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "Entry removed" } }),
    ))
}

fn parse_resource_type(s: &str) -> Result<ResourceType, AppError> {
    match s {
        "file" => Ok(ResourceType::File),
        "folder" => Ok(ResourceType::Folder),
        "storage" => Ok(ResourceType::Storage),
        _ => Err(AppError::validation(format!("Invalid resource type: {s}"))),
    }
}

fn parse_permission(s: &str) -> Result<AclPermission, AppError> {
    match s {
        "owner" => Ok(AclPermission::Owner),
        "editor" => Ok(AclPermission::Editor),
        "commenter" => Ok(AclPermission::Commenter),
        "viewer" => Ok(AclPermission::Viewer),
        _ => Err(AppError::validation(format!("Invalid permission: {s}"))),
    }
}

fn parse_inheritance(s: &str) -> Result<AclInheritance, AppError> {
    match s {
        "inherit" => Ok(AclInheritance::Inherit),
        "block" => Ok(AclInheritance::Block),
        _ => Err(AppError::validation(format!("Invalid inheritance: {s}"))),
    }
}
