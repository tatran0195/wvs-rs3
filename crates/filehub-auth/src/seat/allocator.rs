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

use crate::seat::memory::MemorySeatAllocator;
#[cfg(feature = "redis-seat")]
use crate::seat::redis::RedisSeatAllocator;
use filehub_cache::provider::CacheManager;
use filehub_core::config::SessionConfig;
use filehub_database::repositories::session::SessionRepository;
use std::sync::Arc;

/// Dispatcher for seat allocation strategies.
///
/// Switches between in-memory and Redis-based allocation based on configuration.
#[derive(Debug, Clone)]
pub enum SeatAllocatorDispatch {
    /// In-memory allocator (single node).
    Memory(MemorySeatAllocator),
    /// Redis-based allocator (multi-node).
    #[cfg(feature = "redis-seat")]
    Redis(RedisSeatAllocator),
}

impl SeatAllocatorDispatch {
    /// Creates a new seat allocator dispatcher.
    pub fn new(
        config: &SessionConfig,
        _cache: Arc<CacheManager>,
        _repo: Arc<SessionRepository>,
    ) -> Self {
        // Default to a large number of seats; actual limit is set by LicenseManager
        let total_seats = 1000;
        let reserved = config.admin_reservation.reserved_seats;

        // TODO: inspect cache manager to see if we should use Redis allocator
        // For now, we default to memory allocator to fix compilation.
        // To support Redis properly, we need the Redis URL which isn't exposed by CacheManager currently.

        let allocator = MemorySeatAllocator::new(total_seats, reserved);
        SeatAllocatorDispatch::Memory(allocator)
    }
}

#[async_trait]
impl SeatAllocator for SeatAllocatorDispatch {
    async fn try_allocate(&self, user_key: &str, role: &str) -> Result<AllocationResult, AppError> {
        match self {
            Self::Memory(inner) => inner.try_allocate(user_key, role).await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.try_allocate(user_key, role).await,
        }
    }

    async fn release(&self, user_key: &str) -> Result<(), AppError> {
        match self {
            Self::Memory(inner) => inner.release(user_key).await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.release(user_key).await,
        }
    }

    async fn pool_state(&self) -> Result<PoolState, AppError> {
        match self {
            Self::Memory(inner) => inner.pool_state().await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.pool_state().await,
        }
    }

    async fn set_total_seats(&self, total: u32) -> Result<(), AppError> {
        match self {
            Self::Memory(inner) => inner.set_total_seats(total).await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.set_total_seats(total).await,
        }
    }

    async fn set_admin_reserved(&self, count: u32) -> Result<(), AppError> {
        match self {
            Self::Memory(inner) => inner.set_admin_reserved(count).await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.set_admin_reserved(count).await,
        }
    }

    async fn reconcile(&self, actual_active_sessions: u32) -> Result<(), AppError> {
        match self {
            Self::Memory(inner) => inner.reconcile(actual_active_sessions).await,
            #[cfg(feature = "redis-seat")]
            Self::Redis(inner) => inner.reconcile(actual_active_sessions).await,
        }
    }
}
