//! Session activity tracking middleware — updates `last_activity` on each request.

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::state::AppState;

/// Updates session last_activity timestamp on every authenticated request.
///
/// This is called after auth extraction succeeds, using session ID from JWT claims.
pub async fn track_activity(
    State(_state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Activity tracking happens inside the AuthUser extractor when it validates
    // the session. This middleware is a placeholder for additional activity logic.
    next.run(request).await
}
