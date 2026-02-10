//! User-related domain events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Events related to user operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserEvent {
    /// A new user was created.
    Created {
        /// The user ID.
        user_id: Uuid,
        /// The username.
        username: String,
        /// The assigned role.
        role: String,
    },
    /// A user was updated.
    Updated {
        /// The user ID.
        user_id: Uuid,
        /// Fields that changed.
        changed_fields: Vec<String>,
    },
    /// A user was deleted.
    Deleted {
        /// The user ID.
        user_id: Uuid,
        /// The username.
        username: String,
    },
    /// A user's role was changed.
    RoleChanged {
        /// The user ID.
        user_id: Uuid,
        /// The previous role.
        old_role: String,
        /// The new role.
        new_role: String,
    },
    /// A user's status was changed.
    StatusChanged {
        /// The user ID.
        user_id: Uuid,
        /// The previous status.
        old_status: String,
        /// The new status.
        new_status: String,
    },
    /// A user's password was changed.
    PasswordChanged {
        /// The user ID.
        user_id: Uuid,
    },
    /// A user's account was locked due to failed login attempts.
    AccountLocked {
        /// The user ID.
        user_id: Uuid,
        /// Number of failed attempts.
        failed_attempts: i32,
    },
}
