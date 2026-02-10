//! User role enumeration.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Roles available in the RBAC system.
///
/// Roles are ordered by privilege level: Admin > Manager > Creator > Viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Full system administrator.
    Admin,
    /// Can manage users and storages, but not system config.
    Manager,
    /// Can create files and folders, share content.
    Creator,
    /// Read-only access to shared content.
    Viewer,
}

impl UserRole {
    /// Return the privilege level (higher = more privileged).
    pub fn privilege_level(&self) -> u8 {
        match self {
            Self::Admin => 4,
            Self::Manager => 3,
            Self::Creator => 2,
            Self::Viewer => 1,
        }
    }

    /// Check if this role has at least the given role's privileges.
    pub fn has_at_least(&self, other: &UserRole) -> bool {
        self.privilege_level() >= other.privilege_level()
    }

    /// Check if this role is an admin.
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// Check if this role is a manager or higher.
    pub fn is_manager_or_above(&self) -> bool {
        self.has_at_least(&Self::Manager)
    }

    /// Return the role as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Manager => "manager",
            Self::Creator => "creator",
            Self::Viewer => "viewer",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for UserRole {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "manager" => Ok(Self::Manager),
            "creator" => Ok(Self::Creator),
            "viewer" => Ok(Self::Viewer),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid user role: '{s}'. Expected one of: admin, manager, creator, viewer"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_ordering() {
        assert!(UserRole::Admin.has_at_least(&UserRole::Viewer));
        assert!(UserRole::Admin.has_at_least(&UserRole::Admin));
        assert!(UserRole::Manager.has_at_least(&UserRole::Creator));
        assert!(!UserRole::Viewer.has_at_least(&UserRole::Creator));
    }

    #[test]
    fn test_from_str() {
        assert_eq!("admin".parse::<UserRole>().unwrap(), UserRole::Admin);
        assert_eq!("VIEWER".parse::<UserRole>().unwrap(), UserRole::Viewer);
        assert!("invalid".parse::<UserRole>().is_err());
    }
}
