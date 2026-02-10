//! Job status and priority enumerations.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Waiting to be picked up by a worker.
    Pending,
    /// Placed in a queue, waiting for a worker slot.
    Queued,
    /// Currently being processed by a worker.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed after all retry attempts.
    Failed,
    /// Manually cancelled.
    Cancelled,
}

impl JobStatus {
    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the job can be retried.
    pub fn can_retry(&self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Return the status as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Priority level for a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "job_priority", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    /// Low priority (processed last).
    Low,
    /// Normal priority (default).
    Normal,
    /// High priority.
    High,
    /// Critical priority (processed first).
    Critical,
}

impl JobPriority {
    /// Return the numeric priority (higher = more urgent).
    pub fn numeric_priority(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Normal => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    /// Return the priority as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for JobPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
