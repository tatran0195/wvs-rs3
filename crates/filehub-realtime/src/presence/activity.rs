//! Last activity tracking for idle detection.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;

/// Tracks the last activity timestamp for each user.
#[derive(Debug)]
pub struct LastActivityTracker {
    /// User ID → last activity timestamp.
    activities: DashMap<Uuid, DateTime<Utc>>,
}

impl LastActivityTracker {
    /// Creates a new activity tracker.
    pub fn new() -> Self {
        Self {
            activities: DashMap::new(),
        }
    }

    /// Records activity for a user.
    pub fn record_activity(&self, user_id: Uuid) {
        self.activities.insert(user_id, Utc::now());
    }

    /// Gets the last activity time for a user.
    pub fn last_activity(&self, user_id: &Uuid) -> Option<DateTime<Utc>> {
        self.activities.get(user_id).map(|entry| *entry.value())
    }

    /// Returns users who have been idle for more than the given duration.
    pub fn idle_users(&self, idle_threshold_secs: i64) -> Vec<Uuid> {
        let cutoff = Utc::now() - chrono::Duration::seconds(idle_threshold_secs);
        self.activities
            .iter()
            .filter(|entry| *entry.value() < cutoff)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Removes a user from tracking.
    pub fn remove(&self, user_id: &Uuid) {
        self.activities.remove(user_id);
    }
}

impl Default for LastActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}
