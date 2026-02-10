//! Real-time engine metrics collection.

pub mod channels;
pub mod connections;
pub mod messages;

use std::sync::atomic::{AtomicU64, Ordering};

/// Aggregated real-time engine metrics.
#[derive(Debug)]
pub struct RealtimeMetrics {
    /// Total connections opened since start.
    pub connections_opened: AtomicU64,
    /// Total connections closed since start.
    pub connections_closed: AtomicU64,
    /// Total messages received from clients.
    pub messages_received: AtomicU64,
    /// Total messages sent to clients.
    pub messages_sent: AtomicU64,
}

impl RealtimeMetrics {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self {
            connections_opened: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
        }
    }

    /// Records a connection opened event.
    pub fn connection_opened(&self) {
        self.connections_opened.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a connection closed event.
    pub fn connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a received message.
    pub fn message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Records sent messages.
    pub fn message_sent_count(&self, count: u64) {
        self.messages_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// Returns a snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connections_opened: self.connections_opened.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
        }
    }
}

impl Default for RealtimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Point-in-time metrics snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    /// Total connections opened.
    pub connections_opened: u64,
    /// Total connections closed.
    pub connections_closed: u64,
    /// Total messages received.
    pub messages_received: u64,
    /// Total messages sent.
    pub messages_sent: u64,
}
