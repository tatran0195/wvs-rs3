//! Logging configuration.

use serde::{Deserialize, Serialize};

/// Logging and tracing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`.
    #[serde(default = "default_level")]
    pub level: String,
    /// Log format: `"json"` or `"pretty"`.
    #[serde(default = "default_format")]
    pub format: String,
    /// Path to the application log file.
    #[serde(default = "default_file")]
    pub file: String,
    /// Path to the HTTP access log file.
    #[serde(default = "default_access_log")]
    pub access_log: String,
    /// Maximum log file size in megabytes before rotation.
    #[serde(default = "default_max_size")]
    pub max_file_size_mb: u64,
    /// Maximum number of rotated log files to retain.
    #[serde(default = "default_max_files")]
    pub max_files: u32,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "json".to_string()
}

fn default_file() -> String {
    "data/logs/app.log".to_string()
}

fn default_access_log() -> String {
    "data/logs/access.log".to_string()
}

fn default_max_size() -> u64 {
    100
}

fn default_max_files() -> u32 {
    10
}
