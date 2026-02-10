//! ACL entry entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::acl::{AclInheritance, AclPermission};

/// Resource type for ACL entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "resource_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    /// A file resource.
    File,
    /// A folder resource.
    Folder,
    /// A storage backend resource.
    Storage,
}

impl ResourceType {
    /// Return the type as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Storage => "storage",
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ResourceType {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "file" => Ok(Self::File),
            "folder" => Ok(Self::Folder),
            "storage" => Ok(Self::Storage),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid resource type: '{s}'"
            ))),
        }
    }
}

/// An access control list entry granting a permission to a principal on a resource.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AclEntry {
    /// Unique ACL entry identifier.
    pub id: Uuid,
    /// Type of resource this ACL applies to.
    pub resource_type: ResourceType,
    /// ID of the resource.
    pub resource_id: Uuid,
    /// User granted this permission (None if `is_anyone` is true).
    pub user_id: Option<Uuid>,
    /// Whether this grants public access to anyone.
    pub is_anyone: Option<bool>,
    /// The permission level.
    pub permission: AclPermission,
    /// Inheritance behavior.
    pub inheritance: AclInheritance,
    /// Admin who granted this permission.
    pub granted_by: Uuid,
    /// When this permission expires (None = never).
    pub expires_at: Option<DateTime<Utc>>,
    /// When this entry was created.
    pub created_at: DateTime<Utc>,
}

impl AclEntry {
    /// Check if this ACL entry has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(false)
    }

    /// Check if this is a public access entry.
    pub fn is_public(&self) -> bool {
        self.is_anyone.unwrap_or(false)
    }
}
