//! Connection metrics helpers.

use std::sync::atomic::Ordering;

use super::EngineMetrics;

/// Record a new connection
pub fn record_connect(metrics: &EngineMetrics) {
    metrics.connections_total.fetch_add(1, Ordering::Relaxed);
    metrics.connections_active.fetch_add(1, Ordering::Relaxed);
}

/// Record a disconnection
pub fn record_disconnect(metrics: &EngineMetrics) {
    metrics.connections_active.fetch_sub(1, Ordering::Relaxed);
}
