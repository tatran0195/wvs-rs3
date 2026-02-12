//! Unified error type for the CAD conversion plugin.
//!
//! All subsystem errors (filesystem, scripting, model, process execution)
//! are consolidated into a single `ConversionError` enum that maps cleanly
//! to `filehub_core::error::AppError`.

use filehub_core::error::AppError;
use std::path::PathBuf;
use thiserror::Error;

/// Unified error type for all CAD conversion operations.
#[derive(Debug, Error)]
pub enum ConversionError {
    // --- Filesystem errors ---
    /// ZIP archive contains too many files.
    #[error("ZIP contains {count} files, exceeding limit of {limit}")]
    ZipTooManyFiles {
        /// Actual count of files in the archive.
        count: usize,
        /// Maximum allowed files.
        limit: usize,
    },

    /// ZIP extraction exceeded the total size limit.
    #[error("ZIP extraction exceeded {limit} byte size limit")]
    ZipSizeExceeded {
        /// Maximum allowed bytes.
        limit: u64,
    },

    /// Output parent directory could not be determined.
    #[error("Cannot determine parent directory for: {path}")]
    NoParentDir {
        /// The path whose parent could not be determined.
        path: PathBuf,
    },

    // --- Model errors ---
    /// Import command not implemented for this file type.
    #[error("Import not supported for file type: {file_type}")]
    ImportNotSupported {
        /// String representation of the unsupported file type.
        file_type: String,
    },

    /// Primary file was not specified when required.
    #[error("Primary file not specified (required for Assembly/Combine mode)")]
    PrimaryNotSpecified,

    /// Primary file was not found among resolved inputs.
    #[error("Primary file '{name}' not found in inputs")]
    PrimaryNotFound {
        /// The name that was searched for.
        name: String,
    },

    // --- Script errors ---
    /// No inputs provided for script generation.
    #[error("No inputs provided for script generation")]
    NoInputs,

    // --- Process execution errors ---
    /// Jupiter process timed out.
    #[error("Jupiter process timed out after {timeout_seconds}s")]
    JupiterTimeout {
        /// The timeout duration that was exceeded.
        timeout_seconds: u64,
    },

    /// Jupiter process exited with a non-zero status.
    #[error("Jupiter exited with code {code}: {stderr}")]
    JupiterFailed {
        /// The exit code.
        code: i32,
        /// Captured stderr output.
        stderr: String,
        /// Captured stdout output.
        stdout: String,
    },

    /// Jupiter process was killed or terminated by signal.
    #[error("Jupiter process was killed (signal termination)")]
    JupiterKilled,

    /// Jupiter executable not found at configured path.
    #[error("Jupiter executable not found: {path}")]
    JupiterNotFound {
        /// The configured path that doesn't exist.
        path: PathBuf,
    },

    /// Output file was not created after successful Jupiter execution.
    #[error("Output file not created: {path}")]
    OutputNotCreated {
        /// Expected output path.
        path: PathBuf,
    },

    /// Output file is empty (0 bytes) — likely a Jupiter failure.
    #[error("Output file is empty (0 bytes): {path}")]
    OutputEmpty {
        /// Path to the empty output file.
        path: PathBuf,
    },

    /// Conversion was cancelled via cancellation token.
    #[error("Conversion was cancelled")]
    Cancelled,

    /// Server is at capacity — no conversion slots available.
    #[error("Server at capacity: all {max_slots} conversion slots are in use")]
    AtCapacity {
        /// Total number of conversion slots.
        max_slots: usize,
    },

    /// Semaphore was closed unexpectedly.
    #[error("Internal semaphore error: {reason}")]
    SemaphoreClosed {
        /// Description of which semaphore failed.
        reason: String,
    },

    // --- Generic errors ---
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP library error.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Tokio task join error.
    #[error("Task join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    /// Script path contains invalid UTF-8.
    #[error("Path is not valid UTF-8: {path}")]
    InvalidUtf8Path {
        /// The path that is not valid UTF-8.
        path: PathBuf,
    },
}

impl From<ConversionError> for AppError {
    fn from(err: ConversionError) -> Self {
        match &err {
            ConversionError::AtCapacity { .. } => AppError::service_unavailable(err.to_string()),
            ConversionError::Cancelled => AppError::conflict(err.to_string()),
            ConversionError::PrimaryNotSpecified
            | ConversionError::PrimaryNotFound { .. }
            | ConversionError::NoInputs => AppError::bad_request(err.to_string()),
            ConversionError::JupiterNotFound { .. } => AppError::internal(err.to_string()),
            _ => AppError::internal(err.to_string()),
        }
    }
}
