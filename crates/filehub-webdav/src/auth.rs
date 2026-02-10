//! WebDAV Basic authentication → FileHub session integration.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::UserId;
use filehub_entity::user::model::User;
use filehub_entity::user::role::UserRole;

/// Extracted credentials from a Basic auth header
#[derive(Debug, Clone)]
pub struct BasicCredentials {
    /// Username
    pub username: String,
    /// Password (plaintext from header)
    pub password: String,
}

/// Authenticated WebDAV user context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavUser {
    /// User ID
    pub id: UserId,
    /// Username
    pub username: String,
    /// User role
    pub role: UserRole,
}

impl DavUser {
    /// Create from a User entity
    pub fn from_user(user: &User) -> Self {
        Self {
            id: UserId::from(user.id),
            username: user.username.clone(),
            role: user.role.clone(),
        }
    }

    /// Check if the user has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }
}

/// Extract Basic credentials from HTTP headers
pub fn extract_basic_credentials(headers: &HeaderMap) -> Result<BasicCredentials, AuthError> {
    let auth_header = headers
        .get(http::header::AUTHORIZATION)
        .ok_or(AuthError::MissingHeader)?;

    let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidHeader)?;

    if !auth_str.starts_with("Basic ") {
        return Err(AuthError::NotBasicAuth);
    }

    let encoded = &auth_str[6..];
    let decoded = BASE64
        .decode(encoded)
        .map_err(|_| AuthError::InvalidEncoding)?;

    let decoded_str = String::from_utf8(decoded).map_err(|_| AuthError::InvalidEncoding)?;

    let (username, password) = decoded_str
        .split_once(':')
        .ok_or(AuthError::InvalidFormat)?;

    Ok(BasicCredentials {
        username: username.to_string(),
        password: password.to_string(),
    })
}

/// Build a 401 response with WWW-Authenticate header
pub fn unauthorized_response(realm: &str) -> http::Response<String> {
    http::Response::builder()
        .status(http::StatusCode::UNAUTHORIZED)
        .header(
            http::header::WWW_AUTHENTICATE,
            format!("Basic realm=\"{}\"", realm),
        )
        .body("Unauthorized".to_string())
        .unwrap_or_else(|_| {
            let mut resp = http::Response::new("Unauthorized".to_string());
            *resp.status_mut() = http::StatusCode::UNAUTHORIZED;
            resp
        })
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// No Authorization header present
    #[error("Missing Authorization header")]
    MissingHeader,

    /// Authorization header is not valid UTF-8
    #[error("Invalid Authorization header")]
    InvalidHeader,

    /// Not a Basic auth scheme
    #[error("Not Basic authentication")]
    NotBasicAuth,

    /// Base64 decoding failed
    #[error("Invalid base64 encoding")]
    InvalidEncoding,

    /// Credentials format invalid (missing colon)
    #[error("Invalid credentials format")]
    InvalidFormat,

    /// Authentication failed (bad username or password)
    #[error("Authentication failed")]
    AuthenticationFailed,

    /// User account is locked or inactive
    #[error("Account is locked or inactive")]
    AccountLocked,
}

impl From<AuthError> for AppError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::MissingHeader | AuthError::NotBasicAuth => {
                AppError::unauthorized("Authentication required")
            }
            AuthError::AuthenticationFailed => AppError::unauthorized("Invalid credentials"),
            AuthError::AccountLocked => AppError::forbidden("Account is locked or inactive"),
            _ => AppError::unauthorized("Invalid authentication"),
        }
    }
}
