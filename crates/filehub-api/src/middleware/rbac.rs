//! RBAC middleware helpers for role-based route guarding.

use filehub_core::error::AppError;
use filehub_entity::user::UserRole;

use crate::extractors::AuthUser;

/// Checks that the authenticated user has the Admin role.
pub fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if auth.role != UserRole::Admin {
        return Err(AppError::forbidden("Admin access required"));
    }
    Ok(())
}

/// Checks that the authenticated user has at least Manager role.
pub fn require_manager(auth: &AuthUser) -> Result<(), AppError> {
    match auth.role {
        UserRole::Admin | UserRole::Manager => Ok(()),
        _ => Err(AppError::forbidden("Manager or Admin access required")),
    }
}

/// Checks that the authenticated user has at least Creator role.
pub fn require_creator(auth: &AuthUser) -> Result<(), AppError> {
    match auth.role {
        UserRole::Admin | UserRole::Manager | UserRole::Creator => Ok(()),
        _ => Err(AppError::forbidden(
            "Creator, Manager, or Admin access required",
        )),
    }
}
