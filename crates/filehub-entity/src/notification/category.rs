//! Notification category enumeration.

use serde::{Deserialize, Serialize};

/// Category of a notification for filtering and preference matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    /// File-related notifications (upload, delete, etc.).
    File,
    /// Folder-related notifications.
    Folder,
    /// Share-related notifications.
    Share,
    /// Session-related notifications.
    Session,
    /// System-level notifications.
    System,
    /// Admin broadcast messages.
    Broadcast,
    /// License-related notifications.
    License,
    /// Job completion notifications.
    Job,
}

impl NotificationCategory {
    /// Return the category as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Share => "share",
            Self::Session => "session",
            Self::System => "system",
            Self::Broadcast => "broadcast",
            Self::License => "license",
            Self::Job => "job",
        }
    }
}

impl std::fmt::Display for NotificationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
