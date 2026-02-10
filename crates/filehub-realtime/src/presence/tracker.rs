//! Central presence tracker — maintains online user state.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::activity::LastActivityTracker;
use super::status::PresenceStatus;

/// Information about an online user's presence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    /// User ID.
    pub user_id: Uuid,
    /// Username.
    pub username: String,
    /// Current presence status.
    pub status: PresenceStatus,
    /// When they came online.
    pub online_since: chrono::DateTime<chrono::Utc>,
}

/// Tracks online users and their presence status.
#[derive(Debug)]
pub struct PresenceTracker {
    /// Online users.
    online: DashMap<Uuid, UserPresence>,
    /// Activity tracker.
    activity: LastActivityTracker,
}

impl PresenceTracker {
    /// Creates a new presence tracker.
    pub fn new() -> Self {
        Self {
            online: DashMap::new(),
            activity: LastActivityTracker::new(),
        }
    }

    /// Marks a user as online.
    pub fn set_online(&self, user_id: Uuid, username: String) {
        self.online.insert(
            user_id,
            UserPresence {
                user_id,
                username,
                status: PresenceStatus::Active,
                online_since: chrono::Utc::now(),
            },
        );
        self.activity.record_activity(user_id);
    }

    /// Marks a user as offline.
    pub fn set_offline(&self, user_id: Uuid) {
        self.online.remove(&user_id);
        self.activity.remove(&user_id);
    }

    /// Updates a user's presence status.
    pub fn update_status(&self, user_id: Uuid, status: String) {
        if let Some(mut presence) = self.online.get_mut(&user_id) {
            presence.status = PresenceStatus::from_str_or_default(&status);
        }
        self.activity.record_activity(user_id);
    }

    /// Records activity (resets idle timer).
    pub fn record_activity(&self, user_id: Uuid) {
        self.activity.record_activity(user_id);
    }

    /// Returns all online users.
    pub fn online_users(&self) -> Vec<UserPresence> {
        self.online
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Checks if a user is online.
    pub fn is_online(&self, user_id: &Uuid) -> bool {
        self.online.contains_key(user_id)
    }

    /// Returns the count of online users.
    pub fn online_count(&self) -> usize {
        self.online.len()
    }

    /// Gets presence info for a specific user.
    pub fn get_presence(&self, user_id: &Uuid) -> Option<UserPresence> {
        self.online.get(user_id).map(|entry| entry.value().clone())
    }

    /// Returns users that have been idle longer than the threshold.
    pub fn idle_users(&self, idle_threshold_secs: i64) -> Vec<Uuid> {
        self.activity.idle_users(idle_threshold_secs)
    }

    /// Transitions idle users to idle status.
    pub fn mark_idle_users(&self, idle_threshold_secs: i64) {
        let idle = self.activity.idle_users(idle_threshold_secs);
        for user_id in &idle {
            if let Some(mut presence) = self.online.get_mut(user_id) {
                if presence.status == PresenceStatus::Active {
                    presence.status = PresenceStatus::Idle;
                }
            }
        }
    }
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self::new()
    }
}
