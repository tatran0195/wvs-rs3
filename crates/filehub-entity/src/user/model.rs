//! User entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::role::UserRole;
use super::status::UserStatus;

/// A registered user in the FileHub system.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    /// Unique user identifier.
    pub id: Uuid,
    /// Unique login name.
    pub username: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// Argon2 password hash.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// User role (RBAC).
    pub role: UserRole,
    /// Account status.
    pub status: UserStatus,
    /// Number of consecutive failed login attempts.
    pub failed_login_attempts: Option<i32>,
    /// Account locked until this time (if locked).
    pub locked_until: Option<DateTime<Utc>>,
    /// When the user was created.
    pub created_at: DateTime<Utc>,
    /// When the user was last updated.
    pub updated_at: DateTime<Utc>,
    /// Last successful login time.
    pub last_login_at: Option<DateTime<Utc>>,
    /// The admin who created this user.
    pub created_by: Option<Uuid>,
}

impl User {
    /// Check if the user account is currently locked.
    pub fn is_locked(&self) -> bool {
        if self.status == UserStatus::Locked {
            return true;
        }
        if let Some(locked_until) = self.locked_until {
            return Utc::now() < locked_until;
        }
        false
    }

    /// Check if the user can log in right now.
    pub fn can_login(&self) -> bool {
        self.status.can_login() && !self.is_locked()
    }

    /// Check if this user has admin privileges.
    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }
}

/// Data required to create a new user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    /// Desired username.
    pub username: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// Pre-hashed password.
    pub password_hash: String,
    /// Display name (optional).
    pub display_name: Option<String>,
    /// Assigned role.
    pub role: UserRole,
    /// Creating admin's user ID (optional).
    pub created_by: Option<Uuid>,
}

/// Data for updating an existing user's profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    /// The user ID to update.
    pub id: Uuid,
    /// New email address.
    pub email: Option<String>,
    /// New display name.
    pub display_name: Option<String>,
}
