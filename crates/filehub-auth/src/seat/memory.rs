//! In-memory seat allocator using Tokio mutex for single-node deployments.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{info, warn};

use filehub_core::error::AppError;

use super::allocator::{AllocationResult, PoolState, SeatAllocator};

/// Internal state for the memory-based seat allocator.
#[derive(Debug)]
struct InnerState {
    /// Total seats available.
    total_seats: u32,
    /// Set of user keys that currently hold a seat.
    allocated: HashSet<String>,
    /// Number of seats reserved for admin users.
    admin_reserved: u32,
}

/// In-memory seat allocator using a Tokio mutex for thread safety.
///
/// Suitable for single-node deployments only.
#[derive(Debug, Clone)]
pub struct MemorySeatAllocator {
    /// Protected inner state.
    state: Arc<Mutex<InnerState>>,
}

impl MemorySeatAllocator {
    /// Creates a new memory-based seat allocator.
    pub fn new(total_seats: u32, admin_reserved: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(InnerState {
                total_seats,
                allocated: HashSet::new(),
                admin_reserved,
            })),
        }
    }
}

#[async_trait]
impl SeatAllocator for MemorySeatAllocator {
    async fn try_allocate(&self, user_key: &str, role: &str) -> Result<AllocationResult, AppError> {
        let mut state = self.state.lock().await;

        let checked_out = state.allocated.len() as u32;
        let total = state.total_seats;
        let reserved = state.admin_reserved;

        // If this user already has a seat, allow (idempotent)
        if state.allocated.contains(user_key) {
            return Ok(AllocationResult::Granted);
        }

        // Calculate available seats
        let is_admin = role == "admin" || role == "Admin";

        let available = if is_admin {
            // Admins can use reserved seats
            total.saturating_sub(checked_out)
        } else {
            // Non-admins cannot use reserved seats
            total.saturating_sub(checked_out).saturating_sub(reserved)
        };

        if available == 0 {
            if is_admin && total > checked_out {
                // Admin using reserved seat
                info!(user_key = %user_key, "Admin using reserved seat");
                state.allocated.insert(user_key.to_string());
                return Ok(AllocationResult::Granted);
            }

            let reason = if is_admin {
                "All seats are occupied (including admin reserved)"
            } else {
                "All available seats are occupied (some seats reserved for administrators)"
            };

            return Ok(AllocationResult::Denied {
                reason: reason.to_string(),
            });
        }

        state.allocated.insert(user_key.to_string());
        info!(
            user_key = %user_key,
            checked_out = state.allocated.len(),
            total = total,
            "Seat allocated"
        );

        Ok(AllocationResult::Granted)
    }

    async fn release(&self, user_key: &str) -> Result<(), AppError> {
        let mut state = self.state.lock().await;

        if state.allocated.remove(user_key) {
            info!(
                user_key = %user_key,
                checked_out = state.allocated.len(),
                "Seat released"
            );
        } else {
            warn!(user_key = %user_key, "Attempted to release seat that was not allocated");
        }

        Ok(())
    }

    async fn pool_state(&self) -> Result<PoolState, AppError> {
        let state = self.state.lock().await;
        let checked_out = state.allocated.len() as u32;

        Ok(PoolState {
            total_seats: state.total_seats,
            checked_out,
            available: state.total_seats.saturating_sub(checked_out),
            admin_reserved: state.admin_reserved,
            active_sessions: checked_out,
        })
    }

    async fn set_total_seats(&self, total: u32) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        state.total_seats = total;
        info!(total = total, "Total seats updated");
        Ok(())
    }

    async fn set_admin_reserved(&self, count: u32) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        state.admin_reserved = count;
        info!(count = count, "Admin reserved seats updated");
        Ok(())
    }

    async fn reconcile(&self, actual_active_sessions: u32) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        let pool_count = state.allocated.len() as u32;

        if pool_count != actual_active_sessions {
            warn!(
                pool_count = pool_count,
                db_count = actual_active_sessions,
                "Drift detected between pool and database, reconciling"
            );
            // We can't perfectly reconcile in memory mode since we don't know
            // which keys are stale. We clear and let sessions re-register.
            // In practice, this is triggered on startup.
            state.allocated.clear();
        }

        Ok(())
    }
}
