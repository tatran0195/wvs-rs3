//! Concurrent session seat allocation and pool management.
//!
//! Provides atomic seat allocation using either:
//! - Redis Lua scripts (for multi-node deployments)
//! - In-memory mutex (for single-node deployments)

pub mod allocator;
pub mod limiter;
pub mod memory;
pub mod reconciler;
#[cfg(feature = "redis-seat")]
pub mod redis;

pub use allocator::{AllocationResult, SeatAllocator};
pub use limiter::SessionLimiter;
pub use reconciler::SeatReconciler;
