//! Pool state reconciliation between the seat allocator and the database.
//!
//! Detects and corrects drift caused by crashes, network partitions, or bugs.

use std::sync::Arc;

use tracing::{error, info, warn};

use filehub_core::error::AppError;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;

use super::allocator::SeatAllocator;

use crate::session::store::SessionStore;

/// Reconciles the seat allocator pool state with database reality.
#[derive(Clone)]
pub struct SeatReconciler {
    /// Seat allocator to reconcile.
    allocator: Arc<dyn SeatAllocator>,
    /// Session store for querying actual session counts.
    session_store: Arc<SessionStore>,
    /// Pool snapshot repository for recording state.
    snapshot_repo: Arc<PoolSnapshotRepository>,
}

impl std::fmt::Debug for SeatReconciler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeatReconciler").finish()
    }
}

impl SeatReconciler {
    /// Creates a new seat reconciler.
    pub fn new(
        allocator: Arc<dyn SeatAllocator>,
        session_store: Arc<SessionStore>,
        snapshot_repo: Arc<PoolSnapshotRepository>,
    ) -> Self {
        Self {
            allocator,
            session_store,
            snapshot_repo,
        }
    }

    /// Performs a full reconciliation cycle:
    ///
    /// 1. Query actual active session count from database.
    /// 2. Query pool state from allocator.
    /// 3. Detect drift.
    /// 4. If drift detected, force pool state to match database.
    /// 5. Record a pool snapshot.
    pub async fn reconcile(&self) -> Result<bool, AppError> {
        let db_active = self.session_store.count_all_active().await? as u32;
        let pool_state = self.allocator.pool_state().await?;

        let drift_detected = pool_state.checked_out != db_active;

        if drift_detected {
            warn!(
                pool_checked_out = pool_state.checked_out,
                db_active_sessions = db_active,
                delta = pool_state.checked_out as i64 - db_active as i64,
                "Pool drift detected, reconciling"
            );

            self.allocator.reconcile(db_active).await?;

            info!("Pool reconciliation completed");
        }

        // Record snapshot

        let total_seats = pool_state.total_seats as i32;
        let checked_out = if drift_detected {
            db_active as i32
        } else {
            pool_state.checked_out as i32
        };
        let available = pool_state.total_seats.saturating_sub(db_active) as i32;
        let admin_reserved = pool_state.admin_reserved as i32;
        let active_sessions = db_active as i32;
        let drift_detail = if drift_detected {
            Some(&serde_json::json!({
                "pool_checked_out": pool_state.checked_out,
                "db_active_sessions": db_active,
                "delta": pool_state.checked_out as i64 - db_active as i64
            }))
        } else {
            None
        };

        if let Err(e) = self
            .snapshot_repo
            .create(
                total_seats,
                checked_out,
                available,
                admin_reserved,
                active_sessions,
                drift_detected,
                drift_detail,
                "reconciler",
            )
            .await
        {
            error!(error = %e, "Failed to save pool snapshot");
        }

        Ok(drift_detected)
    }

    /// Performs startup recovery by reconciling pool state with the database.
    ///
    /// Should be called once during server startup to recover from crashes.
    pub async fn startup_recovery(&self) -> Result<(), AppError> {
        info!("Running startup pool recovery");

        let drift = self.reconcile().await?;

        if drift {
            info!("Startup recovery corrected pool drift");
        } else {
            info!("Startup recovery: pool state is consistent");
        }

        Ok(())
    }
}
