//! ACL enumeration types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Permission level for an ACL entry.
///
/// Ordered by privilege: Owner > Editor > Commenter > Viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "acl_permission", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AclPermission {
    /// Full control including sharing and deleting.
    Owner,
    /// Can edit content and metadata.
    Editor,
    /// Can add comments but not modify.
    Commenter,
    /// Read-only access.
    Viewer,
}

impl AclPermission {
    /// Return the privilege level (higher = more privileged).
    pub fn privilege_level(&self) -> u8 {
        match self {
            Self::Owner => 4,
            Self::Editor => 3,
            Self::Commenter => 2,
            Self::Viewer => 1,
        }
    }

    /// Check if this permission grants at least the given level.
    pub fn has_at_least(&self, required: &AclPermission) -> bool {
        self.privilege_level() >= required.privilege_level()
    }

    /// Check if this permission allows write operations.
    pub fn can_write(&self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }

    /// Check if this permission allows delete operations.
    pub fn can_delete(&self) -> bool {
        matches!(self, Self::Owner)
    }

    /// Check if this permission allows sharing.
    pub fn can_share(&self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }

    /// Return the permission as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Commenter => "commenter",
            Self::Viewer => "viewer",
        }
    }
}

impl fmt::Display for AclPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AclPermission {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(Self::Owner),
            "editor" => Ok(Self::Editor),
            "commenter" => Ok(Self::Commenter),
            "viewer" => Ok(Self::Viewer),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid ACL permission: '{s}'"
            ))),
        }
    }
}

/// Inheritance behavior for ACL entries on folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "acl_inheritance", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AclInheritance {
    /// Permission is inherited by child folders and files.
    Inherit,
    /// Permission inheritance is blocked at this level.
    Block,
}

impl AclInheritance {
    /// Return the inheritance mode as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Block => "block",
        }
    }
}

impl fmt::Display for AclInheritance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
