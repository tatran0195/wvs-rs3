//! JWT authentication middleware (tower layer).

// Authentication is handled via the `AuthUser` extractor.
// This module provides a tower layer for routes that need blanket auth.

use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::state::AppState;

/// Middleware that rejects requests without a valid Authorization header.
///
/// This is a lightweight check — full validation happens in the `AuthUser` extractor.
pub async fn require_auth<B>(request: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    let has_auth = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer "))
        .unwrap_or(false);

    if !has_auth {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
