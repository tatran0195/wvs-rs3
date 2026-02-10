//! JWT authentication for WebSocket connections.

use std::sync::Arc;

use tracing;

use filehub_auth::jwt::claims::Claims;
use filehub_auth::jwt::decoder::JwtDecoder;
use filehub_core::error::AppError;
use filehub_core::types::id::{SessionId, UserId};
use filehub_entity::user::role::UserRole;

/// Authenticated WebSocket user
#[derive(Debug, Clone)]
pub struct WsAuthUser {
    /// User ID
    pub user_id: UserId,
    /// Session ID
    pub session_id: SessionId,
    /// Username
    pub username: String,
    /// User role
    pub role: UserRole,
}

/// Authenticate a WebSocket connection from a JWT token.
///
/// Token is typically passed as a query parameter: `/ws?token={jwt}`
pub async fn authenticate_ws(
    token: &str,
    decoder: &Arc<JwtDecoder>,
) -> Result<WsAuthUser, AppError> {
    let claims = decoder.decode(token).await.map_err(|e| {
        tracing::warn!("WS auth failed: {}", e);
        AppError::unauthorized("Invalid or expired token")
    })?;

    Ok(WsAuthUser {
        user_id: UserId::from(claims.user_id),
        session_id: SessionId::from(claims.session_id),
        username: claims.username,
        role: claims.role,
    })
}
