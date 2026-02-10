//! User account status enumeration.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Account status for a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// Account is active and can log in.
    Active,
    /// Account is deactivated by an admin.
    Inactive,
    /// Account is locked due to failed login attempts.
    Locked,
}

impl UserStatus {
    /// Check if the user can log in with this status.
    pub fn can_login(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Return the status as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Locked => "locked",
        }
    }
}

impl fmt::Display for UserStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for UserStatus {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "locked" => Ok(Self::Locked),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid user status: '{s}'. Expected one of: active, inactive, locked"
            ))),
        }
    }
}
