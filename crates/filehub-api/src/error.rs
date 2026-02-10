//! Maps domain `AppError` to HTTP responses.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use filehub_core::error::{AppError, ErrorKind};

/// Standard API error response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// Machine-readable error code.
    pub error: String,
    /// Human-readable message.
    pub message: String,
    /// Optional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self.kind {
            ErrorKind::Validation => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            ErrorKind::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            ErrorKind::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            ErrorKind::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ErrorKind::Conflict => (StatusCode::CONFLICT, "CONFLICT"),
            ErrorKind::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            ErrorKind::ServiceUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE")
            }
            ErrorKind::Internal => {
                tracing::error!(error = %self.message, "Internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        };

        let body = ApiErrorResponse {
            error: error_code.to_string(),
            message: self.message.clone(),
            details: None,
        };

        (status, Json(body)).into_response()
    }
}
