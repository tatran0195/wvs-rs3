//! Per-user session limit override entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A per-user override for the concurrent session limit.
///
/// When present, this takes priority over the role-based limit
/// from configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSessionLimit {
    /// The user whose limit is overridden.
    pub user_id: Uuid,
    /// Maximum number of concurrent sessions.
    pub max_sessions: i32,
    /// Reason for the override.
    pub reason: Option<String>,
    /// The admin who set this override.
    pub set_by: Option<Uuid>,
    /// When the override was created.
    pub created_at: Option<DateTime<Utc>>,
    /// When the override was last updated.
    pub updated_at: Option<DateTime<Utc>>,
}
