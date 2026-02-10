//! Audit log entry entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// An immutable audit log entry recording a user action.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogEntry {
    /// Unique audit entry identifier.
    pub id: Uuid,
    /// The user who performed the action.
    pub actor_id: Uuid,
    /// The action that was performed (e.g., `"file.upload"`, `"session.terminate"`).
    pub action: String,
    /// The type of target resource (e.g., `"file"`, `"user"`, `"session"`).
    pub target_type: String,
    /// The target resource ID (if applicable).
    pub target_id: Option<Uuid>,
    /// Additional details about the action (JSON).
    pub details: Option<serde_json::Value>,
    /// IP address of the actor.
    pub ip_address: Option<String>,
    /// User-Agent of the actor.
    pub user_agent: Option<String>,
    /// When the action occurred.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogEntry {
    /// The user who performed the action.
    pub actor_id: Uuid,
    /// The action performed.
    pub action: String,
    /// Target resource type.
    pub target_type: String,
    /// Target resource ID.
    pub target_id: Option<Uuid>,
    /// Additional details.
    pub details: Option<serde_json::Value>,
    /// Actor's IP address.
    pub ip_address: Option<String>,
    /// Actor's User-Agent.
    pub user_agent: Option<String>,
}
