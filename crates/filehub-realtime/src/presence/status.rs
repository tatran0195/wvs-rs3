//! Presence status definitions.

use serde::{Deserialize, Serialize};

/// User presence status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    /// User is active and recently interacted.
    Active,
    /// User is connected but idle.
    Idle,
    /// User has marked themselves as away.
    Away,
    /// Do not disturb.
    DoNotDisturb,
    /// User is not connected.
    Offline,
}

impl PresenceStatus {
    /// Parses from a string with a default fallback.
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "idle" => Self::Idle,
            "away" => Self::Away,
            "dnd" | "do_not_disturb" => Self::DoNotDisturb,
            "offline" => Self::Offline,
            _ => Self::Active,
        }
    }

    /// Converts to string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Away => "away",
            Self::DoNotDisturb => "dnd",
            Self::Offline => "offline",
        }
    }
}
