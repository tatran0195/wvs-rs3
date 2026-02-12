//! Conversion metrics and telemetry.
//!
//! Tracks conversion counts, durations, and failure rates for observability.
//! Thread-safe via atomics for counters and a mutex for histograms.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Conversion metrics collector.
///
/// All operations are thread-safe and suitable for concurrent access
/// from multiple conversion tasks.
#[derive(Debug)]
pub struct ConversionMetrics {
    /// Total conversions started.
    pub conversions_started: AtomicU64,
    /// Total conversions completed successfully.
    pub conversions_succeeded: AtomicU64,
    /// Total conversions that failed.
    pub conversions_failed: AtomicU64,
    /// Total conversions that timed out.
    pub conversions_timed_out: AtomicU64,
    /// Total conversions that were cancelled.
    pub conversions_cancelled: AtomicU64,
    /// Total VTFx pass-through files handled.
    pub vtfx_passthrough_count: AtomicU64,
    /// Total bytes of output produced.
    pub total_output_bytes: AtomicU64,
    /// Total bytes of input processed.
    pub total_input_bytes: AtomicU64,
    /// Recent conversion durations (kept for P50/P95/P99 calculations).
    duration_samples: Mutex<Vec<Duration>>,
}

/// Maximum number of duration samples to keep in memory.
const MAX_DURATION_SAMPLES: usize = 1000;

impl ConversionMetrics {
    /// Create a new empty metrics collector.
    pub fn new() -> Self {
        Self {
            conversions_started: AtomicU64::new(0),
            conversions_succeeded: AtomicU64::new(0),
            conversions_failed: AtomicU64::new(0),
            conversions_timed_out: AtomicU64::new(0),
            conversions_cancelled: AtomicU64::new(0),
            vtfx_passthrough_count: AtomicU64::new(0),
            total_output_bytes: AtomicU64::new(0),
            total_input_bytes: AtomicU64::new(0),
            duration_samples: Mutex::new(Vec::with_capacity(MAX_DURATION_SAMPLES)),
        }
    }

    /// Record a conversion start.
    pub fn record_started(&self) {
        self.conversions_started.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful conversion with its duration and output size.
    pub fn record_success(&self, duration: Duration, output_bytes: u64) {
        self.conversions_succeeded.fetch_add(1, Ordering::Relaxed);
        self.total_output_bytes
            .fetch_add(output_bytes, Ordering::Relaxed);
        self.add_duration_sample(duration);
    }

    /// Record a failed conversion.
    pub fn record_failure(&self) {
        self.conversions_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timed-out conversion.
    pub fn record_timeout(&self) {
        self.conversions_timed_out.fetch_add(1, Ordering::Relaxed);
        self.conversions_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cancelled conversion.
    pub fn record_cancelled(&self) {
        self.conversions_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a VTFx pass-through operation.
    pub fn record_vtfx_passthrough(&self, bytes: u64) {
        self.vtfx_passthrough_count.fetch_add(1, Ordering::Relaxed);
        self.total_output_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Add a duration sample, evicting the oldest if at capacity.
    fn add_duration_sample(&self, duration: Duration) {
        if let Ok(mut samples) = self.duration_samples.lock() {
            if samples.len() >= MAX_DURATION_SAMPLES {
                samples.remove(0);
            }
            samples.push(duration);
        }
    }

    /// Get the current snapshot of metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let durations = self
            .duration_samples
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();

        let (p50, p95, p99) = Self::calculate_percentiles(&durations);

        MetricsSnapshot {
            conversions_started: self.conversions_started.load(Ordering::Relaxed),
            conversions_succeeded: self.conversions_succeeded.load(Ordering::Relaxed),
            conversions_failed: self.conversions_failed.load(Ordering::Relaxed),
            conversions_timed_out: self.conversions_timed_out.load(Ordering::Relaxed),
            conversions_cancelled: self.conversions_cancelled.load(Ordering::Relaxed),
            vtfx_passthrough_count: self.vtfx_passthrough_count.load(Ordering::Relaxed),
            total_output_bytes: self.total_output_bytes.load(Ordering::Relaxed),
            total_input_bytes: self.total_input_bytes.load(Ordering::Relaxed),
            duration_p50: p50,
            duration_p95: p95,
            duration_p99: p99,
            sample_count: durations.len() as u64,
        }
    }

    /// Calculate P50/P95/P99 from a sorted list of durations.
    fn calculate_percentiles(
        durations: &[Duration],
    ) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
        if durations.is_empty() {
            return (None, None, None);
        }

        let mut sorted = durations.to_vec();
        sorted.sort();
        let len = sorted.len();

        let p50 = sorted.get(len * 50 / 100).copied();
        let p95 = sorted.get(len * 95 / 100).copied();
        let p99 = sorted.get(len.saturating_sub(1) * 99 / 100).copied();

        (p50, p95, p99)
    }
}

impl Default for ConversionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// A point-in-time snapshot of conversion metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    /// Total conversions started.
    pub conversions_started: u64,
    /// Total successful conversions.
    pub conversions_succeeded: u64,
    /// Total failed conversions.
    pub conversions_failed: u64,
    /// Total timed-out conversions.
    pub conversions_timed_out: u64,
    /// Total cancelled conversions.
    pub conversions_cancelled: u64,
    /// Total VTFx pass-through operations.
    pub vtfx_passthrough_count: u64,
    /// Total output bytes produced.
    pub total_output_bytes: u64,
    /// Total input bytes processed.
    pub total_input_bytes: u64,
    /// P50 conversion duration.
    #[serde(
        serialize_with = "serialize_opt_duration",
        deserialize_with = "deserialize_opt_duration"
    )]
    pub duration_p50: Option<Duration>,
    /// P95 conversion duration.
    #[serde(
        serialize_with = "serialize_opt_duration",
        deserialize_with = "deserialize_opt_duration"
    )]
    pub duration_p95: Option<Duration>,
    /// P99 conversion duration.
    #[serde(
        serialize_with = "serialize_opt_duration",
        deserialize_with = "deserialize_opt_duration"
    )]
    pub duration_p99: Option<Duration>,
    /// Number of duration samples collected.
    pub sample_count: u64,
}

/// Serialize an optional Duration as milliseconds.
fn serialize_opt_duration<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match duration {
        Some(d) => serializer.serialize_some(&d.as_millis()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an optional Duration from milliseconds.
fn deserialize_opt_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis: Option<u128> = serde::Deserialize::deserialize(deserializer)?;
    Ok(millis.map(|ms| Duration::from_millis(ms as u64)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counting() {
        let m = ConversionMetrics::new();
        m.record_started();
        m.record_started();
        m.record_success(Duration::from_secs(5), 1000);
        m.record_failure();

        let snap = m.snapshot();
        assert_eq!(snap.conversions_started, 2);
        assert_eq!(snap.conversions_succeeded, 1);
        assert_eq!(snap.conversions_failed, 1);
        assert_eq!(snap.total_output_bytes, 1000);
    }

    #[test]
    fn test_metrics_percentiles() {
        let m = ConversionMetrics::new();
        for i in 1..=100 {
            m.record_success(Duration::from_millis(i * 10), 100);
        }

        let snap = m.snapshot();
        assert!(snap.duration_p50.is_some());
        assert!(snap.duration_p95.is_some());
        assert!(snap.duration_p99.is_some());

        let p50 = snap.duration_p50.expect("p50");
        let p95 = snap.duration_p95.expect("p95");
        assert!(p95 > p50);
    }

    #[test]
    fn test_metrics_empty_percentiles() {
        let m = ConversionMetrics::new();
        let snap = m.snapshot();
        assert!(snap.duration_p50.is_none());
        assert!(snap.duration_p95.is_none());
        assert!(snap.duration_p99.is_none());
    }

    #[test]
    fn test_timeout_increments_both() {
        let m = ConversionMetrics::new();
        m.record_timeout();
        let snap = m.snapshot();
        assert_eq!(snap.conversions_timed_out, 1);
        assert_eq!(snap.conversions_failed, 1);
    }

    #[test]
    fn test_snapshot_serialization() {
        let m = ConversionMetrics::new();
        m.record_success(Duration::from_secs(3), 500);
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).expect("serialize");
        let deser: MetricsSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.conversions_succeeded, 1);
    }
}
