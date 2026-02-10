//! Channel type definitions and parsing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Typed channel identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum ChannelType {
    /// Personal user channel — notifications, session events.
    User(Uuid),
    /// Folder channel — file changes in a folder.
    Folder(Uuid),
    /// File channel — file-specific events (lock, version).
    File(Uuid),
    /// Upload progress channel.
    Upload(Uuid),
    /// Job progress channel.
    Job(Uuid),
    /// Admin session monitoring channel.
    AdminSessions,
    /// Admin system events channel.
    AdminSystem,
    /// Global broadcast channel (all users).
    BroadcastAll,
    /// Global presence channel.
    PresenceGlobal,
}

impl ChannelType {
    /// Parses a channel string into a typed channel.
    pub fn parse(channel: &str) -> Option<Self> {
        let parts: Vec<&str> = channel.splitn(2, ':').collect();
        match parts.as_slice() {
            ["user", id] => Uuid::parse_str(id).ok().map(ChannelType::User),
            ["folder", id] => Uuid::parse_str(id).ok().map(ChannelType::Folder),
            ["file", id] => Uuid::parse_str(id).ok().map(ChannelType::File),
            ["upload", id] => Uuid::parse_str(id).ok().map(ChannelType::Upload),
            ["job", id] => Uuid::parse_str(id).ok().map(ChannelType::Job),
            ["admin", "sessions"] => Some(ChannelType::AdminSessions),
            ["admin", "system"] => Some(ChannelType::AdminSystem),
            ["broadcast", "all"] => Some(ChannelType::BroadcastAll),
            ["presence", "global"] => Some(ChannelType::PresenceGlobal),
            _ => None,
        }
    }

    /// Converts back to a channel string.
    pub fn to_channel_string(&self) -> String {
        match self {
            ChannelType::User(id) => format!("user:{id}"),
            ChannelType::Folder(id) => format!("folder:{id}"),
            ChannelType::File(id) => format!("file:{id}"),
            ChannelType::Upload(id) => format!("upload:{id}"),
            ChannelType::Job(id) => format!("job:{id}"),
            ChannelType::AdminSessions => "admin:sessions".to_string(),
            ChannelType::AdminSystem => "admin:system".to_string(),
            ChannelType::BroadcastAll => "broadcast:all".to_string(),
            ChannelType::PresenceGlobal => "presence:global".to_string(),
        }
    }
}
