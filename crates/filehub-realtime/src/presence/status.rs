//! Presence status definitions.

use serde::{Deserialize, Serialize};

/// User presence status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    /// User is actively using the system
    Active,
    /// User is connected but idle
    Idle,
    /// User has set themselves as away
    Away,
    /// Do not disturb
    Dnd,
    /// User is offline (no connections)
    Offline,
}

impl PresenceStatus {
    /// Parse from string
    pub fn from_str_value(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "idle" => Self::Idle,
            "away" => Self::Away,
            "dnd" => Self::Dnd,
            "offline" => Self::Offline,
            _ => Self::Active,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Away => "away",
            Self::Dnd => "dnd",
            Self::Offline => "offline",
        }
    }
}

impl std::fmt::Display for PresenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
