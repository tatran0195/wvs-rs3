//! Channel type definitions and parsing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All supported channel types in the system.
///
/// Channel names follow the format `{type}:{id}` or `{type}:{scope}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    /// Per-user private channel: `user:{user_id}`
    /// Receives: notifications, session events
    User(Uuid),

    /// Folder watch channel: `folder:{folder_id}`
    /// Receives: file created/updated/deleted, folder changes
    Folder(Uuid),

    /// File watch channel: `file:{file_id}`
    /// Receives: file updated, version created, lock/unlock, comments
    File(Uuid),

    /// Upload progress channel: `upload:{upload_id}`
    /// Receives: chunk progress, completion, errors
    Upload(Uuid),

    /// Job progress channel: `job:{job_id}`
    /// Receives: job started, progress, completed, failed
    Job(Uuid),

    /// Admin sessions channel: `admin:sessions`
    /// Receives: session created/terminated, seat changes
    AdminSessions,

    /// Admin system channel: `admin:system`
    /// Receives: system health, worker status, storage alerts
    AdminSystem,

    /// Global broadcast channel: `broadcast:all`
    /// Receives: admin broadcasts, system announcements
    BroadcastAll,

    /// Global presence channel: `presence:global`
    /// Receives: user online/offline/status changes
    PresenceGlobal,

    /// Storage events channel: `storage:{storage_id}`
    /// Receives: storage status changes, sync events, quota alerts
    Storage(Uuid),

    /// Share events channel: `share:{share_id}`
    /// Receives: share accessed, download count, expiry warnings
    Share(Uuid),
}

impl ChannelType {
    /// Parse a channel name string into a ChannelType.
    ///
    /// Format: `{type}:{id_or_scope}`
    pub fn parse(channel: &str) -> Option<Self> {
        let parts: Vec<&str> = channel.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        match parts[0] {
            "user" => Uuid::parse_str(parts[1]).ok().map(Self::User),
            "folder" => Uuid::parse_str(parts[1]).ok().map(Self::Folder),
            "file" => Uuid::parse_str(parts[1]).ok().map(Self::File),
            "upload" => Uuid::parse_str(parts[1]).ok().map(Self::Upload),
            "job" => Uuid::parse_str(parts[1]).ok().map(Self::Job),
            "storage" => Uuid::parse_str(parts[1]).ok().map(Self::Storage),
            "share" => Uuid::parse_str(parts[1]).ok().map(Self::Share),
            "admin" => match parts[1] {
                "sessions" => Some(Self::AdminSessions),
                "system" => Some(Self::AdminSystem),
                _ => None,
            },
            "broadcast" => match parts[1] {
                "all" => Some(Self::BroadcastAll),
                _ => None,
            },
            "presence" => match parts[1] {
                "global" => Some(Self::PresenceGlobal),
                _ => None,
            },
            _ => None,
        }
    }

    /// Convert to the canonical channel name string
    pub fn to_channel_name(&self) -> String {
        match self {
            Self::User(id) => format!("user:{}", id),
            Self::Folder(id) => format!("folder:{}", id),
            Self::File(id) => format!("file:{}", id),
            Self::Upload(id) => format!("upload:{}", id),
            Self::Job(id) => format!("job:{}", id),
            Self::Storage(id) => format!("storage:{}", id),
            Self::Share(id) => format!("share:{}", id),
            Self::AdminSessions => "admin:sessions".to_string(),
            Self::AdminSystem => "admin:system".to_string(),
            Self::BroadcastAll => "broadcast:all".to_string(),
            Self::PresenceGlobal => "presence:global".to_string(),
        }
    }

    /// Check if this channel requires admin role
    pub fn requires_admin(&self) -> bool {
        matches!(self, Self::AdminSessions | Self::AdminSystem)
    }

    /// Check if this is a user-specific private channel
    pub fn is_user_channel(&self) -> bool {
        matches!(self, Self::User(_))
    }

    /// Check if this channel is open to all authenticated users
    pub fn is_public(&self) -> bool {
        matches!(self, Self::BroadcastAll | Self::PresenceGlobal)
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_channel_name())
    }
}
