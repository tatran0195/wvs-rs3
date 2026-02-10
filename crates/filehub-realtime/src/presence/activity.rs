//! Last activity tracking per user.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;

/// Tracks when each user was last active.
#[derive(Debug)]
pub struct ActivityTracker {
    /// User ID → last activity time
    last_active: DashMap<Uuid, DateTime<Utc>>,
}

impl ActivityTracker {
    /// Create a new activity tracker
    pub fn new() -> Self {
        Self {
            last_active: DashMap::new(),
        }
    }

    /// Record activity for a user
    pub fn record(&self, user_id: Uuid) {
        self.last_active.insert(user_id, Utc::now());
    }

    /// Get last activity time for a user
    pub fn get(&self, user_id: Uuid) -> Option<DateTime<Utc>> {
        self.last_active.get(&user_id).map(|r| *r.value())
    }

    /// Remove a user (on disconnect)
    pub fn remove(&self, user_id: Uuid) {
        self.last_active.remove(&user_id);
    }

    /// Get all users active since the given time
    pub fn active_since(&self, since: DateTime<Utc>) -> Vec<Uuid> {
        self.last_active
            .iter()
            .filter(|r| *r.value() >= since)
            .map(|r| *r.key())
            .collect()
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}
