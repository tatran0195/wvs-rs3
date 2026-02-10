//! Notification priority levels.

use serde::{Deserialize, Serialize};

/// Priority levels for notifications.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    /// Low priority — batch-safe, can be delayed.
    Low,
    /// Normal priority — standard delivery.
    Normal,
    /// High priority — deliver immediately.
    High,
    /// Urgent priority — bypass dedup, deliver immediately.
    Urgent,
}

impl NotificationPriority {
    /// Parses a priority string.
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "urgent" => Self::Urgent,
            _ => Self::Normal,
        }
    }

    /// Converts to string representation.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}
