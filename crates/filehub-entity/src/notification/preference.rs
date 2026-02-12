//! Notification preference entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Per-user notification delivery preferences.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationPreference {
    /// The user these preferences belong to.
    pub user_id: Uuid,
    /// Preferences as a JSON object.
    ///
    /// Structure:
    /// ```json
    /// {
    ///   "file": { "enabled": true, "realtime": true, "email": false },
    ///   "share": { "enabled": true, "realtime": true, "email": true },
    ///   "session": { "enabled": true, "realtime": true, "email": false },
    ///   ...
    /// }
    /// ```
    pub preferences: serde_json::Value,
    /// When preferences were last updated.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Preference settings for a single notification category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPreference {
    /// Whether this category is enabled at all.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether to deliver via real-time WebSocket.
    #[serde(default = "default_true")]
    pub realtime: bool,
    /// Whether to deliver via email.
    #[serde(default)]
    pub email: bool,
}

impl NotificationPreference {
    /// Create default preferences for a user.
    pub fn default_for_user(user_id: Uuid) -> Self {
        Self {
            user_id,
            preferences: serde_json::json!({
                "file": CategoryPreference::default(),
                "share": CategoryPreference::default(),
                "session": CategoryPreference::default(),
                "system": CategoryPreference::default(),
            }),
            updated_at: Some(Utc::now()),
        }
    }
}

impl Default for CategoryPreference {
    fn default() -> Self {
        Self {
            enabled: true,
            realtime: true,
            email: false,
        }
    }
}

fn default_true() -> bool {
    true
}
