//! `AuthUser` extractor — pulls JWT from the Authorization header, validates, and injects context.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use filehub_core::error::AppError;
use filehub_service::context::RequestContext;

use crate::state::AppState;

/// Extracted authenticated user context available in handlers.
#[derive(Debug, Clone)]
pub struct AuthUser(pub RequestContext);

impl AuthUser {
    /// Returns the inner `RequestContext`.
    pub fn context(&self) -> &RequestContext {
        &self.0
    }
}

impl std::ops::Deref for AuthUser {
    type Target = RequestContext;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Bearer token from Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("Invalid Authorization header format"))?;

        // Decode and validate JWT
        let claims = state.jwt_decoder.decode_access_token(token).await?;

        // Validate session is still active
        let _session = state
            .session_manager
            .validate_session(claims.session_id())
            .await?;

        // Extract IP and User-Agent
        let ip_address = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let user_agent = parts
            .headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let ctx = RequestContext::new(
            claims.user_id(),
            claims.session_id(),
            claims.role,
            claims.username,
            ip_address,
            user_agent,
        );

        Ok(AuthUser(ctx))
    }
}
