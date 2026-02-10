//! Admin user management — CRUD, role changes, status changes, password resets.

use std::sync::Arc;

use chrono::Utc;
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
            .find_all_paginated(page)
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

        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            username: req.username.clone(),
            email: req.email,
            password_hash,
            display_name: req.display_name,
            role: req.role.clone(),
            status: UserStatus::Active,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            created_by: Some(ctx.user_id),
        };

        self.user_repo
            .create(&user)
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

        let mut user = self.get_user(ctx, user_id).await?;

        if let Some(display_name) = req.display_name {
            user.display_name = Some(display_name);
        }

        if let Some(email) = req.email {
            if let Some(existing) = self
                .user_repo
                .find_by_email(&email)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            {
                if existing.id != user_id {
                    return Err(AppError::conflict("Email is already in use"));
                }
            }
            user.email = Some(email);
        }

        user.updated_at = Utc::now();

        self.user_repo
            .update(&user)
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

        let mut user = self.get_user(ctx, user_id).await?;
        let old_role = user.role.clone();
        user.role = new_role.clone();
        user.updated_at = Utc::now();

        self.user_repo
            .update(&user)
            .await
            .map_err(|e| AppError::internal(format!("Failed to change role: {e}")))?;

        info!(
            admin_id = %ctx.user_id,
            target_id = %user_id,
            old_role = %old_role,
            new_role = %new_role,
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

        let mut user = self.get_user(ctx, user_id).await?;
        user.status = new_status.clone();
        user.updated_at = Utc::now();

        if new_status == UserStatus::Active {
            user.failed_login_attempts = 0;
            user.locked_until = None;
        }

        self.user_repo
            .update(&user)
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
