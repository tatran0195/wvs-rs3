//! Authentication configuration.

use serde::{Deserialize, Serialize};

/// Authentication and credential configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Secret key for JWT signing (HMAC-SHA256).
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    /// Access token TTL in minutes.
    #[serde(default = "default_access_ttl")]
    pub jwt_access_ttl_minutes: u64,
    /// Refresh token TTL in hours.
    #[serde(default = "default_refresh_ttl")]
    pub jwt_refresh_ttl_hours: u64,
    /// Minimum password length.
    #[serde(default = "default_password_min")]
    pub password_min_length: usize,
    /// Maximum failed login attempts before lockout.
    #[serde(default = "default_max_failed")]
    pub max_failed_attempts: i32,
    /// Account lockout duration in minutes.
    #[serde(default = "default_lockout")]
    pub lockout_duration_minutes: u64,
}

fn default_jwt_secret() -> String {
    "CHANGE_ME_IN_PRODUCTION".to_string()
}

fn default_access_ttl() -> u64 {
    15
}

fn default_refresh_ttl() -> u64 {
    24
}

fn default_password_min() -> usize {
    8
}

fn default_max_failed() -> i32 {
    5
}

fn default_lockout() -> u64 {
    30
}
