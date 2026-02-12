//! Application configuration schemas.
//!
//! All configuration structs are deserialized from TOML files via the
//! `config` crate. Each sub-module represents a logical configuration
//! section.

pub mod app;
pub mod auth;
pub mod cache;
pub mod database;
pub mod license;
pub mod logging;
pub mod plugin;
pub mod realtime;
pub mod session;
pub mod storage;
pub mod worker;

use serde::{Deserialize, Serialize};

pub use self::app::{CorsConfig, ServerConfig};
pub use self::auth::AuthConfig;
pub use self::cache::CacheConfig;
pub use self::database::DatabaseConfig;
pub use self::license::LicenseConfig;
pub use self::logging::LoggingConfig;
pub use self::plugin::PluginConfig;
pub use self::realtime::{NotificationRealtimeConfig, RealtimeConfig};
pub use self::session::SessionConfig;
pub use self::storage::StorageConfig;
pub use self::worker::WorkerConfig;

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
