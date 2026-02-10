//! Application configuration schemas.
//!
//! All configuration structs are deserialized from TOML files via the
//! `config` crate. Each sub-module represents a logical configuration
//! section.

pub mod app;
pub mod auth;
pub mod cache;
pub mod license;
pub mod logging;
pub mod realtime;
pub mod session;
pub mod storage;
pub mod worker;

use serde::{Deserialize, Serialize};

use self::app::ServerConfig;
use self::auth::AuthConfig;
use self::cache::CacheConfig;
use self::license::LicenseConfig;
use self::logging::LoggingConfig;
use self::realtime::RealtimeConfig;
use self::session::SessionConfig;
use self::storage::StorageConfig;
use self::worker::WorkerConfig;

use crate::error::AppError;

/// Root application configuration.
///
/// This struct is the top-level deserialization target for the merged
/// TOML configuration files (default.toml + environment overlay).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Database connection settings.
    pub database: DatabaseConfig,
    /// Cache provider settings.
    pub cache: CacheConfig,
    /// Authentication settings.
    pub auth: AuthConfig,
    /// Session management settings.
    pub session: SessionConfig,
    /// File storage settings.
    pub storage: StorageConfig,
    /// License integration settings.
    pub license: LicenseConfig,
    /// Background worker settings.
    pub worker: WorkerConfig,
    /// Real-time WebSocket settings.
    pub realtime: RealtimeConfig,
    /// Plugin system settings.
    pub plugins: PluginConfig,
    /// Logging settings.
    pub logging: LoggingConfig,
}

/// Database connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL.
    pub url: String,
    /// Maximum number of connections in the pool.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Minimum number of connections in the pool.
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    /// Connection timeout in seconds.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_seconds: u64,
    /// Idle connection timeout in seconds.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_seconds: u64,
}

/// Plugin system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Directory containing plugin shared libraries.
    #[serde(default = "default_plugin_directory")]
    pub directory: String,
    /// Whether to automatically load plugins on startup.
    #[serde(default = "default_true")]
    pub auto_load: bool,
}

impl AppConfig {
    /// Load configuration from TOML files.
    ///
    /// Merges the default configuration with an environment-specific overlay
    /// and environment variables prefixed with `FILEHUB_`.
    pub fn load(env: &str) -> Result<Self, AppError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name(&format!("config/{env}")).required(false))
            .add_source(
                config::Environment::with_prefix("FILEHUB")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| AppError::configuration(format!("Failed to build config: {e}")))?;

        config
            .try_deserialize()
            .map_err(|e| AppError::configuration(format!("Failed to deserialize config: {e}")))
    }
}

fn default_max_connections() -> u32 {
    20
}

fn default_min_connections() -> u32 {
    5
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_idle_timeout() -> u64 {
    300
}

fn default_plugin_directory() -> String {
    "./plugins".to_string()
}

fn default_true() -> bool {
    true
}
