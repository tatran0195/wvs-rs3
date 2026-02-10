//! Request DTOs with validation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Login request body.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    /// Username.
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,
    /// Password.
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Token refresh request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    /// Refresh token.
    pub refresh_token: String,
}

/// Password change request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    /// Current password.
    #[validate(length(min = 1))]
    pub current_password: String,
    /// New password.
    #[validate(length(min = 8))]
    pub new_password: String,
}

/// Update profile request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    /// Display name.
    pub display_name: Option<String>,
    /// Email.
    pub email: Option<String>,
}

/// Create user request (admin).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    /// Username.
    #[validate(length(min = 3, max = 100))]
    pub username: String,
    /// Email.
    pub email: Option<String>,
    /// Password.
    #[validate(length(min = 8))]
    pub password: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Role.
    pub role: String,
}

/// Create folder request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateFolderRequest {
    /// Storage ID.
    pub storage_id: Uuid,
    /// Parent folder ID.
    pub parent_id: Option<Uuid>,
    /// Folder name.
    #[validate(length(min = 1, max = 255))]
    pub name: String,
}

/// Update file request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileRequest {
    /// New name.
    pub name: Option<String>,
    /// New metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Move file request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveFileRequest {
    /// Target folder ID.
    pub target_folder_id: Uuid,
}

/// Copy file request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyFileRequest {
    /// Target folder ID.
    pub target_folder_id: Uuid,
    /// New name.
    pub new_name: Option<String>,
}

/// Initiate chunked upload request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct InitiateUploadRequest {
    /// Target folder ID.
    pub folder_id: Uuid,
    /// File name.
    #[validate(length(min = 1, max = 255))]
    pub file_name: String,
    /// File size in bytes.
    pub file_size: i64,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Expected checksum.
    pub checksum_sha256: Option<String>,
}

/// Create share request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareRequest {
    /// Share type.
    pub share_type: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource ID.
    pub resource_id: Uuid,
    /// Password (optional).
    pub password: Option<String>,
    /// Target user (for user_share).
    pub shared_with: Option<Uuid>,
    /// Permission level.
    pub permission: String,
    /// Allow download.
    #[serde(default = "default_true")]
    pub allow_download: bool,
    /// Max downloads.
    pub max_downloads: Option<i32>,
    /// Expiration.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn default_true() -> bool {
    true
}

/// Update share request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateShareRequest {
    /// Permission.
    pub permission: Option<String>,
    /// Allow download.
    pub allow_download: Option<bool>,
    /// Max downloads.
    pub max_downloads: Option<Option<i32>>,
    /// Expiration.
    pub expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    /// Active state.
    pub is_active: Option<bool>,
}

/// Create ACL entry request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAclEntryRequest {
    /// User ID.
    pub user_id: Option<Uuid>,
    /// Public access.
    #[serde(default)]
    pub is_anyone: bool,
    /// Permission.
    pub permission: String,
    /// Inheritance.
    #[serde(default = "default_inherit")]
    pub inheritance: String,
    /// Expiration.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn default_inherit() -> String {
    "inherit".to_string()
}

/// Terminate session request (admin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateSessionRequest {
    /// Reason.
    pub reason: String,
}

/// Bulk terminate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkTerminateRequest {
    /// Session IDs.
    pub session_ids: Vec<Uuid>,
    /// Reason.
    pub reason: String,
}

/// Admin broadcast request.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BroadcastRequest {
    /// Target ("all" or channel name).
    #[validate(length(min = 1))]
    pub target: String,
    /// Title.
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    /// Message body.
    #[validate(length(min = 1))]
    pub message: String,
    /// Severity (info/warning/error/critical).
    #[validate(length(min = 1))]
    pub severity: String,
    /// Persistent.
    #[serde(default)]
    pub persistent: bool,
}

/// Search files request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilesRequest {
    /// Search query.
    pub query: String,
    /// Folder filter.
    pub folder_id: Option<Uuid>,
    /// Storage filter.
    pub storage_id: Option<Uuid>,
    /// MIME type filter.
    pub mime_type: Option<String>,
    /// Owner filter.
    pub owner_id: Option<Uuid>,
    /// Min size.
    pub min_size: Option<i64>,
    /// Max size.
    pub max_size: Option<i64>,
}

/// Share password verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareVerifyRequest {
    /// Password.
    pub password: String,
}

/// Update notification preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesRequest {
    /// Preferences JSON.
    pub preferences: serde_json::Value,
}

/// Update presence status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePresenceRequest {
    /// Status.
    pub status: String,
}

/// Role change request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRoleRequest {
    /// New role.
    pub role: String,
}

/// Status change request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeStatusRequest {
    /// New status.
    pub status: String,
}

/// Reset password request (admin).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    /// New password.
    #[validate(length(min = 8))]
    pub new_password: String,
}

/// Set user session limit request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetUserLimitRequest {
    /// Max sessions.
    pub max_sessions: u32,
    /// Reason.
    pub reason: Option<String>,
}

/// Send message to session request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendSessionMessageRequest {
    /// Message text.
    pub message: String,
}
