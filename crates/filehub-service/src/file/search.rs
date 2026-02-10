//! Full-text file search with filtering.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::file::FileRepository;
use filehub_entity::file::File;

use crate::context::RequestContext;

/// File search service with full-text and filter support.
#[derive(Debug, Clone)]
pub struct SearchService {
    /// File repository.
    file_repo: Arc<FileRepository>,
}

/// Search request parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchRequest {
    /// Full-text search query.
    pub query: String,
    /// Filter by folder ID.
    pub folder_id: Option<Uuid>,
    /// Filter by storage ID.
    pub storage_id: Option<Uuid>,
    /// Filter by MIME type prefix (e.g., "image/").
    pub mime_type: Option<String>,
    /// Filter by owner.
    pub owner_id: Option<Uuid>,
    /// Minimum file size in bytes.
    pub min_size: Option<i64>,
    /// Maximum file size in bytes.
    pub max_size: Option<i64>,
}

impl SearchService {
    /// Creates a new search service.
    pub fn new(file_repo: Arc<FileRepository>) -> Self {
        Self { file_repo }
    }

    /// Searches files using full-text search and optional filters.
    pub async fn search(
        &self,
        ctx: &RequestContext,
        req: SearchRequest,
        page: PageRequest,
    ) -> Result<PageResponse<File>, AppError> {
        if req.query.trim().is_empty() && req.folder_id.is_none() {
            return Err(AppError::validation(
                "Search query or folder filter is required",
            ));
        }

        self.file_repo
            .search(
                &req.query,
                req.folder_id,
                req.storage_id,
                req.mime_type.as_deref(),
                req.owner_id,
                req.min_size,
                req.max_size,
                ctx.user_id,
                page,
            )
            .await
            .map_err(|e| AppError::internal(format!("Search failed: {e}")))
    }
}
