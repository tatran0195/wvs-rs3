//! Channel metrics helpers.

use std::sync::atomic::Ordering;

use super::EngineMetrics;

/// Record a subscribe operation
pub fn record_subscribe(metrics: &EngineMetrics) {
    metrics.subscriptions_total.fetch_add(1, Ordering::Relaxed);
}
