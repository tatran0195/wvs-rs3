//! Presence tracker — manages user online/offline/status state.

use chrono::Utc;
use dashmap::DashMap;
use uuid::Uuid;

use super::activity::ActivityTracker;
use super::status::PresenceStatus;

use crate::message::types::OutboundMessage;

/// Tracks presence state for all users.
#[derive(Debug)]
pub struct PresenceTracker {
    /// User ID → current status
    statuses: DashMap<Uuid, PresenceStatus>,
    /// User ID → username (cached)
    usernames: DashMap<Uuid, String>,
    /// Activity tracker
    activity: ActivityTracker,
}

impl PresenceTracker {
    /// Create a new presence tracker
    pub fn new() -> Self {
        Self {
            statuses: DashMap::new(),
            usernames: DashMap::new(),
            activity: ActivityTracker::new(),
        }
    }

    /// Mark a user as online
    pub fn set_online(&self, user_id: Uuid, username: &str) -> OutboundMessage {
        self.statuses.insert(user_id, PresenceStatus::Active);
        self.usernames.insert(user_id, username.to_string());
        self.activity.record(user_id);

        OutboundMessage::UserOnline {
            user_id,
            username: username.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Mark a user as offline
    pub fn set_offline(&self, user_id: Uuid) -> OutboundMessage {
        let username = self
            .usernames
            .remove(&user_id)
            .map(|(_, n)| n)
            .unwrap_or_else(|| "unknown".to_string());

        self.statuses.remove(&user_id);
        self.activity.remove(user_id);

        OutboundMessage::UserOffline {
            user_id,
            username,
            timestamp: Utc::now(),
        }
    }

    /// Update a user's status
    pub fn update_status(&self, user_id: Uuid, status: PresenceStatus) -> OutboundMessage {
        let username = self
            .usernames
            .get(&user_id)
            .map(|r| r.value().clone())
            .unwrap_or_else(|| "unknown".to_string());

        self.statuses.insert(user_id, status.clone());
        self.activity.record(user_id);

        OutboundMessage::PresenceChanged {
            user_id,
            username,
            status: status.as_str().to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Get a user's current status
    pub fn get_status(&self, user_id: Uuid) -> PresenceStatus {
        self.statuses
            .get(&user_id)
            .map(|r| r.value().clone())
            .unwrap_or(PresenceStatus::Offline)
    }

    /// Check if a user is online
    pub fn is_online(&self, user_id: Uuid) -> bool {
        self.statuses.contains_key(&user_id)
    }

    /// Get all online users with their statuses
    pub fn all_online(&self) -> Vec<OnlineUser> {
        self.statuses
            .iter()
            .map(|r| {
                let user_id = *r.key();
                let username = self
                    .usernames
                    .get(&user_id)
                    .map(|n| n.value().clone())
                    .unwrap_or_default();
                OnlineUser {
                    user_id,
                    username,
                    status: r.value().clone(),
                }
            })
            .collect()
    }

    /// Get online user count
    pub fn online_count(&self) -> usize {
        self.statuses.len()
    }

    /// Record activity (touch)
    pub fn record_activity(&self, user_id: Uuid) {
        self.activity.record(user_id);
    }
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Online user info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnlineUser {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// Presence status
    pub status: PresenceStatus,
}
