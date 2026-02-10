//! Notification entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A notification to be delivered to a user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    /// Unique notification identifier.
    pub id: Uuid,
    /// The recipient user.
    pub user_id: Uuid,
    /// Notification category.
    pub category: String,
    /// Event type that triggered this notification.
    pub event_type: String,
    /// Notification title.
    pub title: String,
    /// Notification body text.
    pub message: String,
    /// Additional structured data (JSON).
    pub payload: Option<serde_json::Value>,
    /// Priority level.
    pub priority: Option<String>,
    /// Whether the user has read this notification.
    pub is_read: Option<bool>,
    /// When the notification was read.
    pub read_at: Option<DateTime<Utc>>,
    /// Whether the user dismissed this notification.
    pub is_dismissed: Option<bool>,
    /// The user who triggered the action (if applicable).
    pub actor_id: Option<Uuid>,
    /// Resource type involved (if applicable).
    pub resource_type: Option<String>,
    /// Resource ID involved (if applicable).
    pub resource_id: Option<Uuid>,
    /// When the notification was created.
    pub created_at: DateTime<Utc>,
    /// When the notification expires.
    pub expires_at: Option<DateTime<Utc>>,
}

impl Notification {
    /// Check if the notification has been read.
    pub fn is_unread(&self) -> bool {
        !self.is_read.unwrap_or(false)
    }

    /// Check if the notification has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(false)
    }
}
