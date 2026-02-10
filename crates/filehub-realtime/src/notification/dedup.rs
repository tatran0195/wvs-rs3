//! Notification deduplication — batches rapid events within a configurable window.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Key for deduplication grouping.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DedupKey {
    /// Target user ID.
    pub user_id: Uuid,
    /// Event type.
    pub event_type: String,
    /// Resource ID (if applicable).
    pub resource_id: Option<Uuid>,
}

/// Entry in the dedup buffer.
#[derive(Debug, Clone)]
struct DedupEntry {
    /// When this entry was first seen.
    first_seen: DateTime<Utc>,
    /// How many events have been grouped.
    count: u32,
    /// Whether the entry has been flushed.
    flushed: bool,
}

/// Deduplicates rapid notification events within a time window.
#[derive(Debug)]
pub struct NotificationDedup {
    /// Dedup buffer.
    buffer: Arc<Mutex<HashMap<DedupKey, DedupEntry>>>,
    /// Window duration.
    window: Duration,
}

impl NotificationDedup {
    /// Creates a new dedup engine with the given window.
    pub fn new(window_ms: u64) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(HashMap::new())),
            window: Duration::from_millis(window_ms),
        }
    }

    /// Checks whether an event should be delivered or is a duplicate.
    ///
    /// Returns `true` if the event should be sent, `false` if it's a duplicate
    /// within the dedup window.
    pub async fn should_deliver(&self, key: DedupKey) -> bool {
        let mut buffer = self.buffer.lock().await;
        let now = Utc::now();

        if let Some(entry) = buffer.get_mut(&key) {
            let elapsed = (now - entry.first_seen).to_std().unwrap_or(Duration::ZERO);

            if elapsed < self.window {
                // Within window — increment count but don't deliver
                entry.count += 1;
                return false;
            }

            // Window expired — deliver and reset
            entry.first_seen = now;
            entry.count = 1;
            entry.flushed = false;
            return true;
        }

        // New event — record and deliver
        buffer.insert(
            key,
            DedupEntry {
                first_seen: now,
                count: 1,
                flushed: false,
            },
        );

        true
    }

    /// Cleans up expired entries from the buffer.
    pub async fn cleanup(&self) {
        let mut buffer = self.buffer.lock().await;
        let now = Utc::now();
        let window = self.window;

        buffer.retain(|_, entry| {
            let elapsed = (now - entry.first_seen).to_std().unwrap_or(Duration::ZERO);
            elapsed < window * 2
        });
    }
}
