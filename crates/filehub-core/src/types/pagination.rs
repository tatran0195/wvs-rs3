//! Pagination types for list endpoints.

use serde::{Deserialize, Serialize};

/// Default page size.
const DEFAULT_PAGE_SIZE: u64 = 25;
/// Maximum page size.
const MAX_PAGE_SIZE: u64 = 100;

/// Request parameters for paginated queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    /// Page number (1-based).
    #[serde(default = "default_page")]
    pub page: u64,
    /// Number of items per page.
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

impl PageRequest {
    /// Create a new page request.
    pub fn new(page: u64, page_size: u64) -> Self {
        Self {
            page: page.max(1),
            page_size: page_size.clamp(1, MAX_PAGE_SIZE),
        }
    }

    /// Calculate the SQL `OFFSET` value.
    pub fn offset(&self) -> u64 {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// Return the SQL `LIMIT` value.
    pub fn limit(&self) -> u64 {
        self.page_size
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T: Serialize> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Current page number (1-based).
    pub page: u64,
    /// Number of items per page.
    pub page_size: u64,
    /// Total number of items across all pages.
    pub total_items: u64,
    /// Total number of pages.
    pub total_pages: u64,
    /// Whether there is a next page.
    pub has_next: bool,
    /// Whether there is a previous page.
    pub has_previous: bool,
}

impl<T: Serialize> PageResponse<T> {
    /// Create a new paginated response.
    pub fn new(items: Vec<T>, page: u64, page_size: u64, total_items: u64) -> Self {
        let total_pages = if total_items == 0 {
            1
        } else {
            (total_items + page_size - 1) / page_size
        };
        Self {
            items,
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        }
    }

    /// Create an empty response.
    pub fn empty(page_request: &PageRequest) -> Self {
        Self {
            items: Vec::new(),
            page: page_request.page,
            page_size: page_request.page_size,
            total_items: 0,
            total_pages: 1,
            has_next: false,
            has_previous: false,
        }
    }
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    DEFAULT_PAGE_SIZE
}
