//! Message metrics helpers.

use std::sync::atomic::Ordering;

use super::EngineMetrics;

/// Record a message sent to a client
pub fn record_sent(metrics: &EngineMetrics) {
    metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
}

/// Record a message received from a client
pub fn record_received(metrics: &EngineMetrics) {
    metrics.messages_received.fetch_add(1, Ordering::Relaxed);
}

/// Record a notification dispatch
pub fn record_notification(metrics: &EngineMetrics) {
    metrics
        .notifications_dispatched
        .fetch_add(1, Ordering::Relaxed);
}

/// Record a notification persisted for offline user
pub fn record_persisted(metrics: &EngineMetrics) {
    metrics
        .notifications_persisted
        .fetch_add(1, Ordering::Relaxed);
}

/// Record a deduplicated event
pub fn record_deduped(metrics: &EngineMetrics) {
    metrics.events_deduplicated.fetch_add(1, Ordering::Relaxed);
}
