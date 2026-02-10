//! Realtime engine metrics.

pub mod channels;
pub mod connections;
pub mod messages;

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Engine-level metrics counters.
#[derive(Debug)]
pub struct EngineMetrics {
    /// Total messages sent
    pub messages_sent: AtomicU64,
    /// Total messages received
    pub messages_received: AtomicU64,
    /// Total connections established
    pub connections_total: AtomicU64,
    /// Total connections currently active
    pub connections_active: AtomicU64,
    /// Total subscribe operations
    pub subscriptions_total: AtomicU64,
    /// Total notifications dispatched
    pub notifications_dispatched: AtomicU64,
    /// Total notifications persisted (offline)
    pub notifications_persisted: AtomicU64,
    /// Total events deduplicated
    pub events_deduplicated: AtomicU64,
}

impl EngineMetrics {
    /// Create new zeroed metrics
    pub fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            subscriptions_total: AtomicU64::new(0),
            notifications_dispatched: AtomicU64::new(0),
            notifications_persisted: AtomicU64::new(0),
            events_deduplicated: AtomicU64::new(0),
        }
    }

    /// Increment a counter
    pub fn inc_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            subscriptions_total: self.subscriptions_total.load(Ordering::Relaxed),
            notifications_dispatched: self.notifications_dispatched.load(Ordering::Relaxed),
            notifications_persisted: self.notifications_persisted.load(Ordering::Relaxed),
            events_deduplicated: self.events_deduplicated.load(Ordering::Relaxed),
        }
    }
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total connections ever established
    pub connections_total: u64,
    /// Currently active connections
    pub connections_active: u64,
    /// Total subscribe operations
    pub subscriptions_total: u64,
    /// Total notifications dispatched
    pub notifications_dispatched: u64,
    /// Total notifications persisted for offline users
    pub notifications_persisted: u64,
    /// Total events deduplicated
    pub events_deduplicated: u64,
}
