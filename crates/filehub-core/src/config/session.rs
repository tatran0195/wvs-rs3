//! Session management configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Session management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Idle timeout in minutes before a session is considered inactive.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_minutes: u64,
    /// Absolute session timeout in hours (regardless of activity).
    #[serde(default = "default_absolute_timeout")]
    pub absolute_timeout_hours: u64,
    /// WebSocket heartbeat interval in seconds.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_seconds: u64,
    /// Heartbeat timeout in seconds before a session is marked offline.
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_seconds: u64,
    /// Interval for expired session cleanup in minutes.
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_minutes: u64,
    /// Concurrent session limits configuration.
    #[serde(default)]
    pub limits: SessionLimitsConfig,
    /// Admin seat reservation configuration.
    #[serde(default)]
    pub admin_reservation: AdminReservationConfig,
}

/// Concurrent session limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLimitsConfig {
    /// Whether concurrent session limits are enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Overflow strategy when a user exceeds their session limit.
    #[serde(default)]
    pub overflow_strategy: OverflowStrategy,
    /// Per-role session limits. Key is role name, value is max sessions.
    /// A value of `0` means unlimited (bounded only by the license pool).
    #[serde(default = "default_by_role")]
    pub by_role: HashMap<String, u32>,
}

impl Default for SessionLimitsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            overflow_strategy: OverflowStrategy::default(),
            by_role: default_by_role(),
        }
    }
}

/// Strategy applied when a user tries to exceed their session limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowStrategy {
    /// Deny the new login attempt.
    Deny,
    /// Terminate the oldest existing session to make room.
    KickOldest,
    /// Terminate the most idle existing session to make room.
    KickIdle,
}

impl Default for OverflowStrategy {
    fn default() -> Self {
        Self::Deny
    }
}

impl std::fmt::Display for OverflowStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverflowStrategy::Deny => write!(f, "deny"),
            OverflowStrategy::KickOldest => write!(f, "kick_oldest"),
            OverflowStrategy::KickIdle => write!(f, "kick_idle"),
        }
    }
}

/// Admin seat reservation configuration.
///
/// When enabled, a specified number of license seats are reserved
/// exclusively for admin users, guaranteeing they can always log in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminReservationConfig {
    /// Whether admin seat reservation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Number of seats reserved for admin users.
    #[serde(default = "default_reserved_seats")]
    pub reserved_seats: u32,
}

impl Default for AdminReservationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reserved_seats: default_reserved_seats(),
        }
    }
}

fn default_idle_timeout() -> u64 {
    30
}

fn default_absolute_timeout() -> u64 {
    12
}

fn default_heartbeat_interval() -> u64 {
    30
}

fn default_heartbeat_timeout() -> u64 {
    90
}

fn default_cleanup_interval() -> u64 {
    15
}

fn default_true() -> bool {
    true
}

fn default_reserved_seats() -> u32 {
    1
}

fn default_by_role() -> HashMap<String, u32> {
    let mut map = HashMap::new();
    map.insert("admin".to_string(), 1);
    map.insert("manager".to_string(), 1);
    map.insert("creator".to_string(), 1);
    map.insert("viewer".to_string(), 0);
    map
}
