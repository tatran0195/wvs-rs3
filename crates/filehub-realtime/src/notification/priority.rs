//! Notification priority levels.

use serde::{Deserialize, Serialize};

/// Notification priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    /// Low priority — background events
    Low,
    /// Normal priority — standard events
    Normal,
    /// High priority — important events
    High,
    /// Urgent priority — requires immediate attention
    Urgent,
    /// Critical priority — system-level alerts
    Critical,
}

impl NotificationPriority {
    /// Parse from string
    pub fn from_str_value(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "urgent" => Self::Urgent,
            "critical" => Self::Critical,
            _ => Self::Normal,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
            Self::Critical => "critical",
        }
    }

    /// Whether this priority should always be persisted
    pub fn always_persist(&self) -> bool {
        matches!(self, Self::High | Self::Urgent | Self::Critical)
    }

    /// Whether this priority can be batched/deduped
    pub fn can_batch(&self) -> bool {
        matches!(self, Self::Low | Self::Normal)
    }
}
