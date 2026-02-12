//! Admin user management — CRUD, role changes, status changes, password resets.

use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use filehub_auth::password::{PasswordHasher, PasswordValidator};
use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::user::UserRepository;
use filehub_entity::user::{User, UserRole, UserStatus};

use crate::context::RequestContext;

/// Handles administrative user management operations.
#[derive(Debug, Clone)]
pub struct AdminUserService {
    /// User repository.
    user_repo: Arc<UserRepository>,
    /// Password hasher.
    hasher: Arc<PasswordHasher>,
    /// Password validator.
    validator: Arc<PasswordValidator>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

/// Request to create a new user.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateUserRequest {
    /// Username (unique).
    pub username: String,
    /// Email (unique, optional).
    pub email: Option<String>,
    /// Initial password.
    pub password: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Role assignment.
    pub role: UserRole,
}

/// Request to update a user (admin).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateUserRequest {
    /// New display name.
    pub display_name: Option<String>,
    /// New email.
    pub email: Option<String>,
}

impl AdminUserService {
    /// Creates a new admin user service.
    pub fn new(
        user_repo: Arc<UserRepository>,
        hasher: Arc<PasswordHasher>,
        validator: Arc<PasswordValidator>,
        rbac: Arc<RbacEnforcer>,
    ) -> Self {
        Self {
            user_repo,
            hasher,
            validator,
            rbac,
        }
    }

    /// Lists all users with pagination.
    pub async fn list_users(
        &self,
        ctx: &RequestContext,
        page: PageRequest,
    ) -> Result<PageResponse<User>, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserRead)?;

        self.user_repo
            .find_all(&page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list users: {e}")))
    }

    /// Gets a single user by ID.
    pub async fn get_user(&self, ctx: &RequestContext, user_id: Uuid) -> Result<User, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserRead)?;

        self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))
    }

    /// Creates a new user.
    pub async fn create_user(
        &self,
        ctx: &RequestContext,
        req: CreateUserRequest,
    ) -> Result<User, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserCreate)?;

        // Validate username
        if req.username.trim().is_empty() || req.username.len() < 3 {
            return Err(AppError::validation(
                "Username must be at least 3 characters",
            ));
        }

        // Check uniqueness
        if self
            .user_repo
            .find_by_username(&req.username)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .is_some()
        {
            return Err(AppError::conflict("Username is already taken"));
        }

        if let Some(ref email) = req.email {
            if self
                .user_repo
                .find_by_email(email)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
                .is_some()
            {
                return Err(AppError::conflict("Email is already in use"));
            }
        }

        // Validate and hash password
        self.validator.validate(&req.password)?;
        let password_hash = self.hasher.hash_password(&req.password)?;

        let create_data = filehub_entity::user::model::CreateUser {
            username: req.username.clone(),
            email: req.email,
            password_hash,
            display_name: req.display_name,
            role: req.role.clone(),
            created_by: Some(ctx.user_id),
        };

        let user = self
            .user_repo
            .create(&create_data)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create user: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            new_user_id = %user.id,
            username = %user.username,
            role = %user.role,
            "User created by admin"
        );

        Ok(user)
    }

    /// Updates a user's profile fields (admin).
    pub async fn update_user(
        &self,
        ctx: &RequestContext,
        user_id: Uuid,
        req: AdminUpdateUserRequest,
    ) -> Result<User, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserUpdate)?;

        if let Some(ref email) = req.email {
            if let Some(existing) = self
                .user_repo
                .find_by_email(email)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            {
                if existing.id != user_id {
                    return Err(AppError::conflict("Email is already in use"));
                }
            }
        }

        let update_data = filehub_entity::user::model::UpdateUser {
            id: user_id,
            email: req.email,
            display_name: req.display_name,
        };

        let user = self
            .user_repo
            .update(&update_data)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update user: {e}")))?;

        info!(admin_id = %ctx.user_id, target_id = %user_id, "User updated by admin");

        Ok(user)
    }

    /// Changes a user's role.
    pub async fn change_role(
        &self,
        ctx: &RequestContext,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<User, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserChangeRole)?;

        if user_id == ctx.user_id {
            return Err(AppError::forbidden("Cannot change your own role"));
        }

        let user = self
            .user_repo
            .update_role(user_id, new_role.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to change role: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            target_id = %user_id,
            role = %new_role,
            "User role changed"
        );

        Ok(user)
    }

    /// Changes a user's status (active, inactive, locked).
    pub async fn change_status(
        &self,
        ctx: &RequestContext,
        user_id: Uuid,
        new_status: UserStatus,
    ) -> Result<User, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserUpdate)?;

        if user_id == ctx.user_id {
            return Err(AppError::forbidden("Cannot change your own status"));
        }

        if new_status == UserStatus::Active {
            self.user_repo
                .reset_failed_attempts(user_id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to reset attempts: {e}")))?;
        }

        let user = self
            .user_repo
            .update_status(user_id, new_status.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to change status: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            target_id = %user_id,
            new_status = ?new_status,
            "User status changed"
        );

        Ok(user)
    }

    /// Resets a user's password (admin).
    pub async fn reset_password(
        &self,
        ctx: &RequestContext,
        user_id: Uuid,
        new_password: &str,
    ) -> Result<(), AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserResetPassword)?;

        self.validator.validate(new_password)?;
        let hash = self.hasher.hash_password(new_password)?;

        self.user_repo
            .update_password(user_id, &hash)
            .await
            .map_err(|e| AppError::internal(format!("Failed to reset password: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            target_id = %user_id,
            "Password reset by admin"
        );

        Ok(())
    }

    /// Deletes a user.
    pub async fn delete_user(&self, ctx: &RequestContext, user_id: Uuid) -> Result<(), AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::UserDelete)?;

        if user_id == ctx.user_id {
            return Err(AppError::forbidden("Cannot delete your own account"));
        }

        // Ensure user exists
        self.get_user(ctx, user_id).await?;

        self.user_repo
            .delete(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete user: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            target_id = %user_id,
            "User deleted"
        );

        Ok(())
    }
}
