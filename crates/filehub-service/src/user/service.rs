//! User self-service operations — profile viewing and password changes.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::password::{PasswordHasher, PasswordValidator};
use filehub_core::error::AppError;
use filehub_database::repositories::user::UserRepository;
use filehub_entity::user::User;

use crate::context::RequestContext;

/// Handles user self-service operations.
#[derive(Debug, Clone)]
pub struct UserService {
    /// User repository.
    user_repo: Arc<UserRepository>,
    /// Password hasher.
    hasher: Arc<PasswordHasher>,
    /// Password validator.
    validator: Arc<PasswordValidator>,
}

/// Data for updating a user's own profile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateProfileRequest {
    /// New display name (optional).
    pub display_name: Option<String>,
    /// New email (optional).
    pub email: Option<String>,
}

impl UserService {
    /// Creates a new user service.
    pub fn new(
        user_repo: Arc<UserRepository>,
        hasher: Arc<PasswordHasher>,
        validator: Arc<PasswordValidator>,
    ) -> Self {
        Self {
            user_repo,
            hasher,
            validator,
        }
    }

    /// Gets the current user's full profile.
    pub async fn get_profile(&self, ctx: &RequestContext) -> Result<User, AppError> {
        self.user_repo
            .find_by_id(ctx.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))
    }

    /// Updates the current user's profile fields.
    pub async fn update_profile(
        &self,
        ctx: &RequestContext,
        req: UpdateProfileRequest,
    ) -> Result<User, AppError> {
        let mut user = self.get_profile(ctx).await?;

        if let Some(display_name) = req.display_name {
            if display_name.trim().is_empty() {
                return Err(AppError::validation("Display name cannot be empty"));
            }
            user.display_name = Some(display_name);
        }

        if let Some(email) = req.email {
            if !email.contains('@') || !email.contains('.') {
                return Err(AppError::validation("Invalid email format"));
            }

            // Check uniqueness
            if let Some(existing) = self
                .user_repo
                .find_by_email(&email)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            {
                if existing.id != ctx.user_id {
                    return Err(AppError::conflict("Email is already in use"));
                }
            }

            user.email = Some(email);
        }

        user.updated_at = Utc::now();

        self.user_repo
            .update(&user)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update profile: {e}")))?;

        info!(user_id = %ctx.user_id, "Profile updated");

        Ok(user)
    }

    /// Changes the current user's password.
    pub async fn change_password(
        &self,
        ctx: &RequestContext,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        let user = self.get_profile(ctx).await?;

        // Verify current password
        let valid = self
            .hasher
            .verify_password(current_password, &user.password_hash)?;
        if !valid {
            return Err(AppError::unauthorized("Current password is incorrect"));
        }

        // Validate new password
        self.validator.validate(new_password)?;
        self.validator
            .validate_not_same(current_password, new_password)?;

        // Hash and store
        let new_hash = self.hasher.hash_password(new_password)?;

        self.user_repo
            .update_password(ctx.user_id, &new_hash)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update password: {e}")))?;

        info!(user_id = %ctx.user_id, "Password changed");

        Ok(())
    }
}
