//! Seat allocator trait for concurrent session management.

use async_trait::async_trait;

use crate::result::AppResult;
use crate::types::id::{SessionId, UserId};

/// Snapshot of the license/session seat pool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolStatus {
    /// Total available seats in the license pool.
    pub total_seats: u32,
    /// Number of seats currently checked out.
    pub checked_out: u32,
    /// Number of seats available for allocation.
    pub available: u32,
    /// Number of seats reserved for admin users.
    pub admin_reserved: u32,
    /// Number of active sessions in the database.
    pub active_sessions: u32,
}

/// Trait for atomic seat allocation in the license pool.
///
/// Implementations must guarantee atomicity: either a seat is fully
/// allocated or fully rolled back. Two implementations are provided:
/// - Redis-based (using Lua scripts for atomicity)
/// - In-memory (using `tokio::sync::Mutex`)
#[async_trait]
pub trait SeatAllocator: Send + Sync + 'static {
    /// Try to allocate a seat for the given user and session.
    ///
    /// Returns `true` if a seat was successfully allocated.
    async fn try_allocate(
        &self,
        user_id: &UserId,
        session_id: &SessionId,
        is_admin: bool,
    ) -> AppResult<bool>;

    /// Release a previously allocated seat.
    async fn release(&self, user_id: &UserId, session_id: &SessionId) -> AppResult<()>;

    /// Get the current pool status.
    async fn pool_status(&self) -> AppResult<PoolStatus>;

    /// Reconcile the allocator state with the database.
    ///
    /// This corrects any drift between the in-memory/Redis seat count
    /// and the actual number of active sessions in the database.
    async fn reconcile(&self, actual_active_sessions: u32) -> AppResult<()>;

    /// Set the total seat count (e.g., after refreshing from license server).
    async fn set_total_seats(&self, total: u32) -> AppResult<()>;

    /// Set the number of admin-reserved seats.
    async fn set_admin_reserved(&self, reserved: u32) -> AppResult<()>;

    /// Check that the allocator backend is reachable.
    async fn health_check(&self) -> AppResult<bool>;
}
