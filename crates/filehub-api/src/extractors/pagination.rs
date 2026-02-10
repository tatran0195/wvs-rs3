//! Pagination query parameter extractor.

use axum::extract::Query;
use serde::{Deserialize, Serialize};

use filehub_core::types::pagination::PageRequest;

/// Query parameters for paginated endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based, default: 1).
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page (default: 25, max: 100).
    #[serde(default = "default_per_page")]
    pub per_page: u64,
    /// Sort field (optional).
    pub sort_by: Option<String>,
    /// Sort direction: "asc" or "desc".
    pub sort_dir: Option<String>,
}

fn default_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    25
}

impl PaginationParams {
    /// Converts to a `PageRequest`.
    pub fn into_page_request(self) -> PageRequest {
        let per_page = self.per_page.min(100).max(1);
        let page = self.page.max(1);

        PageRequest {
            page,
            per_page,
            sort_by: self.sort_by,
            sort_dir: self.sort_dir,
        }
    }
}
