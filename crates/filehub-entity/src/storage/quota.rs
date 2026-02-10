//! Storage quota value object.

use serde::{Deserialize, Serialize};

/// Quota information for a storage backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    /// Total quota in bytes (None = unlimited).
    pub total_bytes: Option<i64>,
    /// Currently used bytes.
    pub used_bytes: i64,
    /// Available bytes (None if unlimited).
    pub available_bytes: Option<i64>,
    /// Usage percentage (0.0 - 100.0, None if unlimited).
    pub usage_percent: Option<f64>,
}

impl StorageQuota {
    /// Create a quota from total and used values.
    pub fn new(total_bytes: Option<i64>, used_bytes: i64) -> Self {
        let available_bytes = total_bytes.map(|total| (total - used_bytes).max(0));
        let usage_percent = total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (used_bytes as f64 / total as f64) * 100.0
            }
        });

        Self {
            total_bytes,
            used_bytes,
            available_bytes,
            usage_percent,
        }
    }

    /// Check if the quota is exceeded.
    pub fn is_exceeded(&self) -> bool {
        match self.total_bytes {
            Some(total) => self.used_bytes >= total,
            None => false,
        }
    }

    /// Check if adding the given number of bytes would exceed the quota.
    pub fn would_exceed(&self, additional_bytes: i64) -> bool {
        match self.total_bytes {
            Some(total) => (self.used_bytes + additional_bytes) > total,
            None => false,
        }
    }
}
