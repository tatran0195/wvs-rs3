//! License integration configuration.

use serde::{Deserialize, Serialize};

/// License system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseConfig {
    /// Whether license enforcement is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// License provider type (e.g., `"flexnet"`).
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Path to the license file on disk.
    #[serde(default = "default_license_file")]
    pub license_file: String,
    /// Licensed feature name to check out.
    #[serde(default = "default_feature_name")]
    pub feature_name: String,
    /// License pool management configuration.
    #[serde(default)]
    pub pool: LicensePoolConfig,
}

/// License pool management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePoolConfig {
    /// How long to cache pool status in seconds.
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
    /// How often to refresh pool status from the license server.
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_seconds: u64,
    /// Warning threshold as a percentage of pool capacity.
    #[serde(default = "default_warning_threshold")]
    pub warning_threshold_percent: u8,
    /// Critical threshold as a percentage of pool capacity.
    #[serde(default = "default_critical_threshold")]
    pub critical_threshold_percent: u8,
    /// Whether to reserve seats for admins.
    #[serde(default)]
    pub admin_reserved_enabled: bool,
    /// Number of seats to reserve for admins.
    #[serde(default = "default_admin_reserved_seats")]
    pub admin_reserved_seats: u32,
}

impl Default for LicensePoolConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: default_cache_ttl(),
            refresh_interval_seconds: default_refresh_interval(),
            warning_threshold_percent: default_warning_threshold(),
            critical_threshold_percent: default_critical_threshold(),
            admin_reserved_enabled: false,
            admin_reserved_seats: default_admin_reserved_seats(),
        }
    }
}

fn default_admin_reserved_seats() -> u32 {
    2
}

fn default_provider() -> String {
    "flexnet".to_string()
}

fn default_license_file() -> String {
    "data/plugins/flexnet/license.dat".to_string()
}

fn default_feature_name() -> String {
    "suzuki_filehub".to_string()
}

fn default_cache_ttl() -> u64 {
    30
}

fn default_refresh_interval() -> u64 {
    15
}

fn default_warning_threshold() -> u8 {
    80
}

fn default_critical_threshold() -> u8 {
    95
}
