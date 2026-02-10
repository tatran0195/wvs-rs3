//! Deduplication of rapid events within a time window.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Deduplication key
type DedupKey = String;

/// Event deduplicator — batches rapid events within a window.
#[derive(Debug)]
pub struct EventDeduplicator {
    /// Window duration
    window: Duration,
    /// Last seen time per key
    last_seen: Mutex<HashMap<DedupKey, Instant>>,
}

impl EventDeduplicator {
    /// Create a new deduplicator with the given window
    pub fn new(window_ms: u64) -> Self {
        Self {
            window: Duration::from_millis(window_ms),
            last_seen: Mutex::new(HashMap::new()),
        }
    }

    /// Check if an event should be dispatched or deduplicated.
    ///
    /// Returns `true` if the event should proceed, `false` if it's a duplicate.
    pub fn should_dispatch(&self, key: &str) -> bool {
        let mut map = self.last_seen.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        if let Some(last) = map.get(key) {
            if now.duration_since(*last) < self.window {
                return false; // Too recent — suppress
            }
        }

        map.insert(key.to_string(), now);
        true
    }

    /// Build a dedup key from event components
    pub fn make_key(event_type: &str, resource_id: &str, actor_id: &str) -> String {
        format!("{}:{}:{}", event_type, resource_id, actor_id)
    }

    /// Clean up old entries
    pub fn cleanup(&self) {
        let mut map = self.last_seen.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let cutoff = self.window * 10; // Keep entries for 10x the window
        map.retain(|_, v| now.duration_since(*v) < cutoff);
    }
}
