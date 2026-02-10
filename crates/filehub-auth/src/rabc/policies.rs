//! Role-to-permission mapping definitions.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use filehub_entity::user::UserRole;

/// A system-level permission (distinct from ACL resource-level permissions).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPermission {
    // User management
    /// Create new users.
    UserCreate,
    /// Read user profiles.
    UserRead,
    /// Update user details.
    UserUpdate,
    /// Delete users.
    UserDelete,
    /// Change user roles.
    UserChangeRole,
    /// Reset user passwords.
    UserResetPassword,

    // File operations
    /// Upload files.
    FileUpload,
    /// Download files (subject to ACL).
    FileDownload,
    /// Delete files (subject to ACL).
    FileDelete,
    /// Create file versions.
    FileVersion,
    /// Lock/unlock files.
    FileLock,

    // Folder operations
    /// Create folders.
    FolderCreate,
    /// Delete folders.
    FolderDelete,
    /// Move/rename folders.
    FolderManage,

    // Share operations
    /// Create shares.
    ShareCreate,
    /// View shares created by others.
    ShareViewAll,
    /// Manage shares created by others.
    ShareManageAll,

    // Storage management
    /// View storage configurations.
    StorageView,
    /// Add/modify/remove storage backends.
    StorageManage,
    /// Initiate cross-storage transfers.
    StorageTransfer,

    // Session management
    /// View all active sessions.
    SessionViewAll,
    /// Terminate other users' sessions.
    SessionTerminate,
    /// Manage session limits.
    SessionManageLimits,
    /// Send messages to sessions.
    SessionSendMessage,

    // Notification / broadcast
    /// Send admin broadcasts.
    BroadcastSend,

    // License management
    /// View license pool status.
    LicenseView,
    /// Force pool reconciliation.
    LicenseManage,

    // Jobs
    /// View background jobs.
    JobView,
    /// Cancel or retry jobs.
    JobManage,

    // Audit
    /// Search the audit log.
    AuditView,
    /// Export audit logs.
    AuditExport,

    // Reports
    /// View system reports.
    ReportView,

    // System
    /// Access health/status endpoints.
    SystemHealth,
    /// Manage ACL permissions on any resource.
    PermissionManageAll,
}

/// Defines the mapping from each role to its set of allowed system permissions.
#[derive(Debug, Clone)]
pub struct RbacPolicies {
    /// Role → set of permissions.
    policies: HashMap<UserRole, HashSet<SystemPermission>>,
}

impl RbacPolicies {
    /// Creates the default policy set.
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // Viewer: read-only file access
        let mut viewer = HashSet::new();
        viewer.insert(SystemPermission::FileDownload);
        viewer.insert(SystemPermission::StorageView);
        viewer.insert(SystemPermission::SystemHealth);
        policies.insert(UserRole::Viewer, viewer);

        // Creator: viewer + upload, folder create, share
        let mut creator = HashSet::new();
        creator.insert(SystemPermission::FileUpload);
        creator.insert(SystemPermission::FileDownload);
        creator.insert(SystemPermission::FileVersion);
        creator.insert(SystemPermission::FileLock);
        creator.insert(SystemPermission::FolderCreate);
        creator.insert(SystemPermission::ShareCreate);
        creator.insert(SystemPermission::StorageView);
        creator.insert(SystemPermission::SystemHealth);
        policies.insert(UserRole::Creator, creator);

        // Manager: creator + delete, manage folders/shares, view sessions, reports
        let mut manager = HashSet::new();
        manager.insert(SystemPermission::FileUpload);
        manager.insert(SystemPermission::FileDownload);
        manager.insert(SystemPermission::FileDelete);
        manager.insert(SystemPermission::FileVersion);
        manager.insert(SystemPermission::FileLock);
        manager.insert(SystemPermission::FolderCreate);
        manager.insert(SystemPermission::FolderDelete);
        manager.insert(SystemPermission::FolderManage);
        manager.insert(SystemPermission::ShareCreate);
        manager.insert(SystemPermission::ShareViewAll);
        manager.insert(SystemPermission::StorageView);
        manager.insert(SystemPermission::StorageTransfer);
        manager.insert(SystemPermission::UserRead);
        manager.insert(SystemPermission::SessionViewAll);
        manager.insert(SystemPermission::AuditView);
        manager.insert(SystemPermission::ReportView);
        manager.insert(SystemPermission::SystemHealth);
        policies.insert(UserRole::Manager, manager);

        // Admin: everything
        let admin: HashSet<SystemPermission> = vec![
            SystemPermission::UserCreate,
            SystemPermission::UserRead,
            SystemPermission::UserUpdate,
            SystemPermission::UserDelete,
            SystemPermission::UserChangeRole,
            SystemPermission::UserResetPassword,
            SystemPermission::FileUpload,
            SystemPermission::FileDownload,
            SystemPermission::FileDelete,
            SystemPermission::FileVersion,
            SystemPermission::FileLock,
            SystemPermission::FolderCreate,
            SystemPermission::FolderDelete,
            SystemPermission::FolderManage,
            SystemPermission::ShareCreate,
            SystemPermission::ShareViewAll,
            SystemPermission::ShareManageAll,
            SystemPermission::StorageView,
            SystemPermission::StorageManage,
            SystemPermission::StorageTransfer,
            SystemPermission::SessionViewAll,
            SystemPermission::SessionTerminate,
            SystemPermission::SessionManageLimits,
            SystemPermission::SessionSendMessage,
            SystemPermission::BroadcastSend,
            SystemPermission::LicenseView,
            SystemPermission::LicenseManage,
            SystemPermission::JobView,
            SystemPermission::JobManage,
            SystemPermission::AuditView,
            SystemPermission::AuditExport,
            SystemPermission::ReportView,
            SystemPermission::SystemHealth,
            SystemPermission::PermissionManageAll,
        ]
        .into_iter()
        .collect();
        policies.insert(UserRole::Admin, admin);

        Self { policies }
    }

    /// Returns the set of permissions for the given role.
    pub fn permissions_for_role(&self, role: &UserRole) -> HashSet<SystemPermission> {
        self.policies.get(role).cloned().unwrap_or_default()
    }

    /// Checks whether the given role has the specified permission.
    pub fn has_permission(&self, role: &UserRole, permission: &SystemPermission) -> bool {
        self.policies
            .get(role)
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }
}

impl Default for RbacPolicies {
    fn default() -> Self {
        Self::new()
    }
}
