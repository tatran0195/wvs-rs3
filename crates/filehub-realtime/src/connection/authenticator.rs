//! WebSocket authentication — validates JWT from query parameter or first message.

use std::sync::Arc;

use uuid::Uuid;

use filehub_auth::jwt::{Claims, JwtDecoder};
use filehub_core::error::AppError;
use filehub_entity::user::UserRole;

/// Authenticated connection info extracted from JWT.
#[derive(Debug, Clone)]
pub struct AuthenticatedConnection {
    /// User ID.
    pub user_id: Uuid,
    /// Session ID.
    pub session_id: Uuid,
    /// User role.
    pub role: UserRole,
    /// Username.
    pub username: String,
}

/// Authenticates WebSocket connections using JWT tokens.
#[derive(Clone)]
pub struct WsAuthenticator {
    /// JWT decoder.
    decoder: Arc<JwtDecoder>,
}

impl std::fmt::Debug for WsAuthenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsAuthenticator").finish()
    }
}

impl WsAuthenticator {
    /// Creates a new WebSocket authenticator.
    pub fn new(decoder: Arc<JwtDecoder>) -> Self {
        Self { decoder }
    }

    /// Authenticates a connection using a JWT token (typically from query parameter).
    pub async fn authenticate(&self, token: &str) -> Result<AuthenticatedConnection, AppError> {
        let claims = self.decoder.decode_access_token(token).await?;

        Ok(AuthenticatedConnection {
            user_id: claims.user_id(),
            session_id: claims.session_id(),
            role: claims.role,
            username: claims.username,
        })
    }
}
