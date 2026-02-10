//! Convenience result type alias for FileHub.

use crate::error::AppError;

/// A specialized `Result` type for FileHub operations.
///
/// This is defined as a convenience so that every crate does not need to
/// write `Result<T, AppError>` explicitly.
pub type AppResult<T> = Result<T, AppError>;
