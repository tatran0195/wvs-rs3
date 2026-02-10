//! Background worker configuration.

use serde::{Deserialize, Serialize};

/// Background job worker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Whether the worker is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Number of concurrent job processing tasks.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Interval in seconds between job queue polls.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_concurrency() -> usize {
    4
}

fn default_poll_interval() -> u64 {
    5
}
