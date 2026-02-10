//! Response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standard success response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// Whether the request was successful.
    pub success: bool,
    /// Response data.
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    /// Creates a successful response.
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Total item count.
    pub total: u64,
    /// Current page.
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total pages.
    pub total_pages: u64,
}

/// Login response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    /// Access token.
    pub access_token: String,
    /// Refresh token.
    pub refresh_token: String,
    /// Access token expiration.
    pub access_expires_at: DateTime<Utc>,
    /// Refresh token expiration.
    pub refresh_expires_at: DateTime<Utc>,
    /// User info.
    pub user: UserResponse,
}

/// User summary for responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// User ID.
    pub id: Uuid,
    /// Username.
    pub username: String,
    /// Email.
    pub email: Option<String>,
    /// Display name.
    pub display_name: Option<String>,
    /// Role.
    pub role: String,
    /// Status.
    pub status: String,
    /// Created at.
    pub created_at: DateTime<Utc>,
    /// Last login.
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Simple message response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Message.
    pub message: String,
}

/// Count response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    /// Count value.
    pub count: i64,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status.
    pub status: String,
    /// Version.
    pub version: String,
    /// Uptime.
    pub uptime_seconds: u64,
}

/// Detailed health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedHealthResponse {
    /// Overall status.
    pub status: String,
    /// Database status.
    pub database: String,
    /// Cache status.
    pub cache: String,
    /// Storage status.
    pub storage: String,
    /// WebSocket connections.
    pub ws_connections: usize,
    /// Online users.
    pub online_users: usize,
}
