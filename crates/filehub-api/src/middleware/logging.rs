//! Request/response logging middleware.

use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
use tracing::info;

/// Logs request method, path, status, and duration.
pub async fn request_logging<B>(request: Request<B>, next: Next<B>) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        path = %uri.path(),
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "HTTP request"
    );

    response
}
