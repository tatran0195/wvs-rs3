//! RBAC enforcement logic — checks whether a role has a required system permission.

use filehub_core::error::AppError;
use filehub_entity::user::UserRole;

use super::policies::{RbacPolicies, SystemPermission};

/// Enforces role-based access control for system-level operations.
#[derive(Debug, Clone)]
pub struct RbacEnforcer {
    /// The policy configuration.
    policies: RbacPolicies,
}

impl RbacEnforcer {
    /// Creates a new enforcer with the default policy set.
    pub fn new() -> Self {
        Self {
            policies: RbacPolicies::new(),
        }
    }

    /// Creates an enforcer with custom policies.
    pub fn with_policies(policies: RbacPolicies) -> Self {
        Self { policies }
    }

    /// Checks whether the given role has the required permission.
    ///
    /// Returns `Ok(())` if allowed, or `Err(AppError::Forbidden)` if denied.
    pub fn require_permission(
        &self,
        role: &UserRole,
        permission: &SystemPermission,
    ) -> Result<(), AppError> {
        if self.policies.has_permission(role, permission) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "Role '{role}' does not have permission '{permission:?}'"
            )))
        }
    }

    /// Checks whether the role has the required permission (returns bool).
    pub fn has_permission(&self, role: &UserRole, permission: &SystemPermission) -> bool {
        self.policies.has_permission(role, permission)
    }

    /// Checks whether the given role is at least the specified minimum role.
    ///
    /// Role hierarchy: Admin > Manager > Creator > Viewer
    pub fn require_minimum_role(
        &self,
        actual_role: &UserRole,
        minimum_role: &UserRole,
    ) -> Result<(), AppError> {
        if role_level(actual_role) >= role_level(minimum_role) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "Role '{actual_role}' is insufficient; minimum required: '{minimum_role}'"
            )))
        }
    }

    /// Returns whether the role is an admin.
    pub fn is_admin(&self, role: &UserRole) -> bool {
        matches!(role, UserRole::Admin)
    }

    /// Returns a reference to the underlying policies.
    pub fn policies(&self) -> &RbacPolicies {
        &self.policies
    }
}

impl Default for RbacEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps roles to a numeric level for hierarchy comparison.
fn role_level(role: &UserRole) -> u8 {
    match role {
        UserRole::Viewer => 0,
        UserRole::Creator => 1,
        UserRole::Manager => 2,
        UserRole::Admin => 3,
    }
}
