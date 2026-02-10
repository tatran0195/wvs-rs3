//! License pool status and snapshot entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Live status of the license seat pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    /// Total license seats available.
    pub total_seats: i32,
    /// Seats currently checked out.
    pub checked_out: i32,
    /// Seats available for checkout.
    pub available: i32,
    /// Seats reserved for admins.
    pub admin_reserved: i32,
    /// Number of active sessions in the database.
    pub active_sessions: i32,
    /// Whether drift was detected between pool and sessions.
    pub drift_detected: bool,
    /// Usage as a percentage.
    pub usage_percent: f64,
}

impl PoolStatus {
    /// Check if the pool is at warning threshold.
    pub fn is_warning(&self, threshold_percent: u8) -> bool {
        self.usage_percent >= threshold_percent as f64
    }

    /// Check if the pool is at critical threshold.
    pub fn is_critical(&self, threshold_percent: u8) -> bool {
        self.usage_percent >= threshold_percent as f64
    }
}

/// A historical snapshot of the license pool state.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PoolSnapshot {
    /// Snapshot ID.
    pub id: Uuid,
    /// Total seats at snapshot time.
    pub total_seats: i32,
    /// Checked out seats.
    pub checked_out: i32,
    /// Available seats.
    pub available: i32,
    /// Admin reserved seats.
    pub admin_reserved: i32,
    /// Active sessions.
    pub active_sessions: i32,
    /// Whether drift was detected.
    pub drift_detected: Option<bool>,
    /// Details about drift (JSON).
    pub drift_detail: Option<serde_json::Value>,
    /// Source of the snapshot (e.g., "scheduled", "manual", "reconciliation").
    pub source: String,
    /// When the snapshot was taken.
    pub created_at: DateTime<Utc>,
}
