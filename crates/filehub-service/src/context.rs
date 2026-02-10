//! Request context carrying the authenticated user, session, and resolved permissions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use filehub_entity::user::UserRole;

/// Context for the current authenticated request.
///
/// Extracted by middleware and passed into service methods so that
/// every operation knows *who* is acting and from *which* session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// The authenticated user's ID.
    pub user_id: Uuid,
    /// The current session ID.
    pub session_id: Uuid,
    /// The user's role at the time the JWT was issued.
    pub role: UserRole,
    /// The username (convenience field from JWT claims).
    pub username: String,
    /// IP address of the request origin.
    pub ip_address: String,
    /// User-Agent header value.
    pub user_agent: Option<String>,
    /// When the request was received.
    pub request_time: DateTime<Utc>,
}

impl RequestContext {
    /// Creates a new request context.
    pub fn new(
        user_id: Uuid,
        session_id: Uuid,
        role: UserRole,
        username: String,
        ip_address: String,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            user_id,
            session_id,
            role,
            username,
            ip_address,
            user_agent,
            request_time: Utc::now(),
        }
    }

    /// Returns whether the current user is an admin.
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    /// Returns whether the current user is at least a manager.
    pub fn is_manager_or_above(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::Manager)
    }
}
