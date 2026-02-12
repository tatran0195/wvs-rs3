//! Unified application error types for FileHub.
//!
//! All crates map their internal errors into [`AppError`] for consistent
//! propagation through the ? operator.

use std::fmt;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::types::response::ApiErrorResponse;

/// Top-level error kind categorization used across the entire application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ErrorKind {
    /// The requested resource was not found.
    NotFound,
    /// Authentication failed (invalid credentials, expired token, etc.).
    Authentication,
    /// The caller does not have permission to perform the action.
    Authorization,
    /// Input validation failed.
    Validation,
    /// A conflict occurred (duplicate entry, concurrent modification, etc.).
    Conflict,
    /// A rate limit was exceeded.
    RateLimit,
    /// An internal server error occurred.
    Internal,
    /// A database error occurred.
    Database,
    /// A cache error occurred.
    Cache,
    /// A storage I/O error occurred.
    Storage,
    /// A configuration error occurred.
    Configuration,
    /// A license-related error occurred.
    License,
    /// A session-related error occurred.
    Session,
    /// A plugin error occurred.
    Plugin,
    /// A serialization/deserialization error occurred.
    Serialization,
    /// An external service error occurred.
    ExternalService,
    /// The requested feature or operation is not implemented.
    NotImplemented,
    /// The service is temporarily unavailable.
    ServiceUnavailable,
    /// The caller does not have permission to perform the action.
    Forbidden,
    /// The request was malformed or invalid.
    BadRequest,
    /// The caller is not authorized to perform the action.
    Unauthorized,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::Authentication => write!(f, "AUTHENTICATION"),
            Self::Authorization => write!(f, "AUTHORIZATION"),
            Self::Validation => write!(f, "VALIDATION"),
            Self::Conflict => write!(f, "CONFLICT"),
            Self::RateLimit => write!(f, "RATE_LIMIT"),
            Self::Internal => write!(f, "INTERNAL"),
            Self::Database => write!(f, "DATABASE"),
            Self::Cache => write!(f, "CACHE"),
            Self::Storage => write!(f, "STORAGE"),
            Self::Configuration => write!(f, "CONFIGURATION"),
            Self::License => write!(f, "LICENSE"),
            Self::Session => write!(f, "SESSION"),
            Self::Plugin => write!(f, "PLUGIN"),
            Self::Serialization => write!(f, "SERIALIZATION"),
            Self::ExternalService => write!(f, "EXTERNAL_SERVICE"),
            Self::NotImplemented => write!(f, "NOT_IMPLEMENTED"),
            Self::ServiceUnavailable => write!(f, "SERVICE_UNAVAILABLE"),
            Self::Forbidden => write!(f, "FORBIDDEN"),
            Self::BadRequest => write!(f, "BAD_REQUEST"),
            Self::Unauthorized => write!(f, "UNAUTHORIZED"),
        }
    }
}

/// The unified application error used throughout FileHub.
///
/// All crate-specific errors are mapped into `AppError` using `From` impls
/// or explicit `.map_err()` calls. This provides a single error type for
/// the entire application boundary.
#[derive(Debug, Error)]
#[error("{kind}: {message}")]
pub struct AppError {
    /// The category of error.
    pub kind: ErrorKind,
    /// A human-readable error message.
    pub message: String,
    /// Optional underlying cause.
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl AppError {
    /// Create a new application error.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Create a new application error with an underlying cause.
    pub fn with_source(
        kind: ErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a not-found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    /// Create an authentication error.
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authentication, message)
    }

    /// Create an authorization error.
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authorization, message)
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    /// Create a conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    /// Create a database error.
    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Database, message)
    }

    /// Create a cache error.
    pub fn cache(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Cache, message)
    }

    /// Create a storage error.
    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Storage, message)
    }

    /// Create a configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Configuration, message)
    }

    /// Create a license error.
    pub fn license(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::License, message)
    }

    /// Create a session error.
    pub fn session(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Session, message)
    }

    /// Create a plugin error.
    pub fn plugin(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Plugin, message)
    }

    /// Create a not-implemented error.
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotImplemented, message)
    }

    /// Create a service-unavailable error.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ServiceUnavailable, message)
    }

    /// Create a forbidden error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Forbidden, message)
    }

    /// Create a bad request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::BadRequest, message)
    }

    /// Create a rate limit error.
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RateLimit, message)
    }

    /// Create an unauthorized error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unauthorized, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self.kind {
            ErrorKind::Authentication => (StatusCode::UNAUTHORIZED, "AUTHENTICATION"),
            ErrorKind::Authorization => (StatusCode::FORBIDDEN, "AUTHORIZATION"),
            ErrorKind::BadRequest => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            ErrorKind::Validation => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            ErrorKind::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            ErrorKind::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            ErrorKind::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ErrorKind::Conflict => (StatusCode::CONFLICT, "CONFLICT"),
            ErrorKind::RateLimit => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            ErrorKind::Database => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            ErrorKind::Cache => (StatusCode::INTERNAL_SERVER_ERROR, "CACHE_ERROR"),
            ErrorKind::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR"),
            ErrorKind::Configuration => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIGURATION_ERROR"),
            ErrorKind::License => (StatusCode::INTERNAL_SERVER_ERROR, "LICENSE_ERROR"),
            ErrorKind::Session => (StatusCode::INTERNAL_SERVER_ERROR, "SESSION_ERROR"),
            ErrorKind::Plugin => (StatusCode::INTERNAL_SERVER_ERROR, "PLUGIN_ERROR"),
            ErrorKind::Serialization => (StatusCode::INTERNAL_SERVER_ERROR, "SERIALIZATION_ERROR"),
            ErrorKind::ExternalService => {
                (StatusCode::INTERNAL_SERVER_ERROR, "EXTERNAL_SERVICE_ERROR")
            }
            ErrorKind::NotImplemented => (StatusCode::NOT_IMPLEMENTED, "NOT_IMPLEMENTED"),
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

impl Clone for AppError {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            message: self.message.clone(),
            source: None,
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::with_source(
            ErrorKind::Serialization,
            format!("JSON serialization error: {err}"),
            err,
        )
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::with_source(ErrorKind::Storage, format!("I/O error: {err}"), err)
    }
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        Self::with_source(
            ErrorKind::Configuration,
            format!("Configuration error: {err}"),
            err,
        )
    }
}
