//! File CRUD, upload, download handlers.

use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_service::file::upload::{InitiateUploadRequest as SvcInitUpload, SimpleUploadParams};

use crate::dto::request::{
    CopyFileRequest, InitiateUploadRequest, MoveFileRequest, UpdateFileRequest,
};
use crate::dto::response::ApiResponse;
use crate::extractors::{AuthUser, PaginationParams};
use crate::state::AppState;

/// GET /api/files?folder_id=...
pub async fn list_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
    Query(filter): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder_id = filter
        .get("folder_id")
        .ok_or_else(|| AppError::validation("folder_id query parameter is required"))?
        .parse::<Uuid>()
        .map_err(|_| AppError::validation("Invalid folder_id"))?;

    let page = params.into_page_request();
    let result = state
        .file_service
        .list_files(&auth, folder_id, page)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "items": result.items,
            "total": result.total,
            "page": result.page,
            "per_page": result.per_page,
            "total_pages": result.total_pages,
        }
    })))
}

/// GET /api/files/:id
pub async fn get_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state.file_service.get_file(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// GET /api/files/:id/download
pub async fn download_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    let result = state.download_service.download(&auth, id).await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, result.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", result.filename),
        )
        .header(header::CONTENT_LENGTH, result.data.len())
        .body(Body::from(result.data))
        .map_err(|e| AppError::internal(format!("Response build failed: {e}")))?;

    Ok(response)
}

/// GET /api/files/:id/preview
pub async fn preview_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AppError> {
    let size = params.get("size").and_then(|s| s.parse::<u32>().ok());

    let result = state.preview_service.get_preview(&auth, id, size).await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, result.content_type)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(result.data))
        .map_err(|e| AppError::internal(format!("Response build failed: {e}")))?;

    Ok(response)
}

/// GET /api/files/:id/versions
pub async fn list_versions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let versions = state.version_service.list_versions(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": versions }),
    ))
}

/// GET /api/files/:id/versions/:ver
pub async fn download_version(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, ver)): Path<(Uuid, i32)>,
) -> Result<Response, AppError> {
    let result = state
        .download_service
        .download_version(&auth, id, ver)
        .await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, result.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", result.filename),
        )
        .body(Body::from(result.data))
        .map_err(|e| AppError::internal(format!("Response build failed: {e}")))?;

    Ok(response)
}

/// POST /api/files/upload — simple multipart upload
pub async fn simple_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut folder_id: Option<Uuid> = None;
    let mut file_name: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut data: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("Multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "folder_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::validation(format!("Read error: {e}")))?;
                folder_id = Some(
                    Uuid::parse_str(&text)
                        .map_err(|_| AppError::validation("Invalid folder_id"))?,
                );
            }
            "file" => {
                file_name = field.file_name().map(String::from);
                mime_type = field.content_type().map(String::from);
                data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::validation(format!("Read error: {e}")))?,
                );
            }
            _ => {}
        }
    }

    let folder_id = folder_id.ok_or_else(|| AppError::validation("folder_id is required"))?;
    let file_name = file_name.ok_or_else(|| AppError::validation("file is required"))?;
    let data = data.ok_or_else(|| AppError::validation("file data is required"))?;

    let file = state
        .upload_service
        .simple_upload(
            &auth,
            SimpleUploadParams {
                folder_id,
                file_name,
                mime_type,
                data,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// POST /api/files/upload/initiate
pub async fn initiate_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<InitiateUploadRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .upload_service
        .initiate_chunked_upload(
            &auth,
            SvcInitUpload {
                folder_id: req.folder_id,
                file_name: req.file_name,
                file_size: req.file_size,
                mime_type: req.mime_type,
                checksum_sha256: req.checksum_sha256,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}

/// PUT /api/files/upload/:id/chunk/:n
pub async fn upload_chunk(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((upload_id, chunk_n)): Path<(Uuid, i32)>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .upload_service
        .upload_chunk(&auth, upload_id, chunk_n, body)
        .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "chunk": chunk_n } }),
    ))
}

/// POST /api/files/upload/:id/complete
pub async fn complete_upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(upload_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state
        .upload_service
        .complete_chunked_upload(&auth, upload_id)
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// PUT /api/files/:id
pub async fn update_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state
        .file_service
        .update_file(
            &auth,
            id,
            filehub_service::file::service::UpdateFileRequest {
                name: req.name,
                metadata: req.metadata,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// PUT /api/files/:id/move
pub async fn move_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<MoveFileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state
        .file_service
        .move_file(
            &auth,
            id,
            filehub_service::file::service::MoveFileRequest {
                target_folder_id: req.target_folder_id,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// POST /api/files/:id/copy
pub async fn copy_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CopyFileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state
        .file_service
        .copy_file(
            &auth,
            id,
            filehub_service::file::service::CopyFileRequest {
                target_folder_id: req.target_folder_id,
                new_name: req.new_name,
            },
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// DELETE /api/files/:id
pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.file_service.delete_file(&auth, id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "data": { "message": "File deleted" } }),
    ))
}

/// POST /api/files/:id/lock
pub async fn lock_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state.file_service.lock_file(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}

/// POST /api/files/:id/unlock
pub async fn unlock_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let file = state.file_service.unlock_file(&auth, id).await?;
    Ok(Json(serde_json::json!({ "success": true, "data": file })))
}
