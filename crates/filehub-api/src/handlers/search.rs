//! File search handler.

use axum::Json;
use axum::extract::{Query, State};

use filehub_core::error::AppError;
use filehub_service::file::search::SearchRequest;

use crate::dto::request::SearchFilesRequest;
use crate::extractors::{AuthUser, PaginationParams};
use crate::state::AppState;

/// GET /api/files/search
pub async fn search_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
    Query(req): Query<SearchFilesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .search_service
        .search(
            &auth,
            SearchRequest {
                query: req.query,
                folder_id: req.folder_id,
                storage_id: req.storage_id,
                mime_type: req.mime_type,
                owner_id: req.owner_id,
                min_size: req.min_size,
                max_size: req.max_size,
            },
            params.into_page_request(),
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true, "data": result })))
}
