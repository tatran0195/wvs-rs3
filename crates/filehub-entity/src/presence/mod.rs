//! Presence domain entities.

pub mod model;

pub use model::PresenceState;

use serde::{Deserialize, Serialize};

/// Presence status for a user session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "presence_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    /// User is actively interacting.
    Active,
    /// User is logged in but idle.
    Idle,
    /// User is away.
    Away,
    /// User has set Do Not Disturb.
    Dnd,
    /// User is offline.
    Offline,
}

impl PresenceStatus {
    /// Check if the user is considered online.
    pub fn is_online(&self) -> bool {
        !matches!(self, Self::Offline)
    }

    /// Return the status as a lowercase string.
    pub fn as_str(&self) -> &'static str {
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

impl std::str::FromStr for PresenceStatus {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "idle" => Ok(Self::Idle),
            "away" => Ok(Self::Away),
            "dnd" => Ok(Self::Dnd),
            "offline" => Ok(Self::Offline),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid presence status: '{s}'"
            ))),
        }
    }
}
