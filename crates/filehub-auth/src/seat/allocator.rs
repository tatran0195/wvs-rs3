//! Seat allocator trait and shared types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use filehub_core::error::AppError;

/// Result of attempting to allocate a session seat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationResult {
    /// Seat was successfully allocated.
    Granted,
    /// Seat allocation was denied.
    Denied {
        /// Reason for denial.
        reason: String,
    },
}

/// Current state of the session seat pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    /// Total available seats in the pool.
    pub total_seats: u32,
    /// Currently checked-out (allocated) seats.
    pub checked_out: u32,
    /// Available seats (total - checked_out - reserved).
    pub available: u32,
    /// Admin-reserved seats.
    pub admin_reserved: u32,
    /// Number of active sessions in the database.
    pub active_sessions: u32,
}

/// Trait for atomic seat allocation and release.
///
/// Implementations must be thread-safe and handle concurrent access.
#[async_trait]
pub trait SeatAllocator: Send + Sync + std::fmt::Debug {
    /// Attempts to atomically allocate a seat for the given user.
    ///
    /// `user_key` is typically the user ID.
    /// `role` is used for admin reservation checks.
    async fn try_allocate(&self, user_key: &str, role: &str) -> Result<AllocationResult, AppError>;

    /// Releases a previously allocated seat.
    async fn release(&self, user_key: &str) -> Result<(), AppError>;

    /// Returns the current pool state.
    async fn pool_state(&self) -> Result<PoolState, AppError>;

    /// Resets the pool to the given total seat count.
    async fn set_total_seats(&self, total: u32) -> Result<(), AppError>;

    /// Sets the number of admin-reserved seats.
    async fn set_admin_reserved(&self, count: u32) -> Result<(), AppError>;

    /// Forces a reconciliation of the pool state with the database.
    async fn reconcile(&self, actual_active_sessions: u32) -> Result<(), AppError>;
}
