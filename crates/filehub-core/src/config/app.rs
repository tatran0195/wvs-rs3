//! Server, TLS, and CORS configuration.

use serde::{Deserialize, Serialize};

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address.
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Number of worker threads (0 = auto-detect CPU count).
    #[serde(default)]
    pub workers: usize,
    /// Maximum concurrent connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Request timeout in seconds.
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    /// Graceful shutdown timeout in seconds.
    #[serde(default = "default_shutdown_grace")]
    pub shutdown_grace_seconds: u64,
    /// TLS configuration.
    #[serde(default)]
    pub tls: TlsConfig,
    /// CORS configuration.
    #[serde(default)]
    pub cors: CorsConfig,
}

/// TLS termination configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Whether TLS is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Path to the PEM certificate file.
    #[serde(default)]
    pub cert_path: String,
    /// Path to the PEM private key file.
    #[serde(default)]
    pub key_path: String,
}

/// CORS (Cross-Origin Resource Sharing) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (use `["*"]` for development only).
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods.
    #[serde(default = "default_allowed_methods")]
    pub allowed_methods: Vec<String>,
    /// Allowed HTTP headers.
    #[serde(default = "default_allowed_headers")]
    pub allowed_headers: Vec<String>,
    /// Max age for preflight cache in seconds.
    #[serde(default = "default_max_age")]
    pub max_age_seconds: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_allowed_origins(),
            allowed_methods: default_allowed_methods(),
            allowed_headers: default_allowed_headers(),
            max_age_seconds: default_max_age(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_max_connections() -> usize {
    10000
}

fn default_request_timeout() -> u64 {
    30
}

fn default_shutdown_grace() -> u64 {
    30
}

fn default_allowed_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_allowed_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "PATCH".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_allowed_headers() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_max_age() -> u64 {
    3600
}
