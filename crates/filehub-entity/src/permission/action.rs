//! Permission action definitions for RBAC.

use serde::{Deserialize, Serialize};

/// Actions that can be checked against RBAC and ACL policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    // -- File actions --
    /// View/read a file.
    FileRead,
    /// Create a new file.
    FileCreate,
    /// Update file metadata or content.
    FileUpdate,
    /// Delete a file.
    FileDelete,
    /// Download a file.
    FileDownload,
    /// Lock/unlock a file.
    FileLock,
    /// Share a file.
    FileShare,

    // -- Folder actions --
    /// View/read a folder.
    FolderRead,
    /// Create a new folder.
    FolderCreate,
    /// Update a folder.
    FolderUpdate,
    /// Delete a folder.
    FolderDelete,

    // -- User actions --
    /// View user profiles.
    UserRead,
    /// Create a new user.
    UserCreate,
    /// Update a user.
    UserUpdate,
    /// Delete a user.
    UserDelete,
    /// Change a user's role.
    UserChangeRole,

    // -- Storage actions --
    /// View storage details.
    StorageRead,
    /// Create a storage backend.
    StorageCreate,
    /// Update storage configuration.
    StorageUpdate,
    /// Delete a storage backend.
    StorageDelete,

    // -- Session actions --
    /// View active sessions.
    SessionRead,
    /// Terminate a session.
    SessionTerminate,
    /// Manage session limits.
    SessionManageLimits,

    // -- Admin actions --
    /// Access the admin dashboard.
    AdminAccess,
    /// Send admin broadcasts.
    AdminBroadcast,
    /// View audit logs.
    AuditRead,
    /// Manage license pool.
    LicenseManage,
    /// Manage plugins.
    PluginManage,
    /// Manage jobs.
    JobManage,
}

impl PermissionAction {
    /// Return the action as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "file:read",
            Self::FileCreate => "file:create",
            Self::FileUpdate => "file:update",
            Self::FileDelete => "file:delete",
            Self::FileDownload => "file:download",
            Self::FileLock => "file:lock",
            Self::FileShare => "file:share",
            Self::FolderRead => "folder:read",
            Self::FolderCreate => "folder:create",
            Self::FolderUpdate => "folder:update",
            Self::FolderDelete => "folder:delete",
            Self::UserRead => "user:read",
            Self::UserCreate => "user:create",
            Self::UserUpdate => "user:update",
            Self::UserDelete => "user:delete",
            Self::UserChangeRole => "user:change_role",
            Self::StorageRead => "storage:read",
            Self::StorageCreate => "storage:create",
            Self::StorageUpdate => "storage:update",
            Self::StorageDelete => "storage:delete",
            Self::SessionRead => "session:read",
            Self::SessionTerminate => "session:terminate",
            Self::SessionManageLimits => "session:manage_limits",
            Self::AdminAccess => "admin:access",
            Self::AdminBroadcast => "admin:broadcast",
            Self::AuditRead => "audit:read",
            Self::LicenseManage => "license:manage",
            Self::PluginManage => "plugin:manage",
            Self::JobManage => "job:manage",
        }
    }
}

impl std::fmt::Display for PermissionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
