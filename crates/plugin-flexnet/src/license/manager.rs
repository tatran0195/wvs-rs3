//! LicenseManager orchestrates all license operations.
//!
//! Central coordinator that wraps the FFI wrapper and integrates
//! with the database for checkout tracking and pool snapshots.

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing;

use filehub_core::config::LicenseConfig;
use filehub_core::error::AppError;
use filehub_core::types::id::{SessionId, UserId};
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_entity::license::model::LicenseCheckout;
use filehub_entity::license::pool::{PoolSnapshot, PoolStatus};

use crate::ffi::wrapper::LicenseManagerWrapper;

/// Manages all FlexNet license operations.
///
/// Thread-safe — can be shared across handlers via `Arc<LicenseManager>`.
#[derive(Debug)]
pub struct LicenseManager {
    /// FFI wrapper (real or mock)
    wrapper: Arc<LicenseManagerWrapper>,
    /// License configuration
    config: LicenseConfig,
    /// License checkout repository for DB tracking
    checkout_repo: Arc<LicenseCheckoutRepository>,
    /// Pool snapshot repository
    snapshot_repo: Arc<PoolSnapshotRepository>,
    /// Cached pool status with TTL
    cached_status: Arc<RwLock<Option<CachedPoolStatus>>>,
}

/// Cached pool status with expiry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPoolStatus {
    /// The pool status
    status: PoolStatus,
    /// When it was cached
    cached_at: chrono::DateTime<Utc>,
}

impl LicenseManager {
    /// Create a new LicenseManager
    pub fn new(
        wrapper: Arc<LicenseManagerWrapper>,
        config: LicenseConfig,
        checkout_repo: Arc<LicenseCheckoutRepository>,
        snapshot_repo: Arc<PoolSnapshotRepository>,
    ) -> Self {
        Self {
            wrapper,
            config,
            checkout_repo,
            snapshot_repo,
            cached_status: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the license system.
    ///
    /// Loads the DLL (or mock), initializes the context, and syncs pool status.
    pub async fn initialize(&self) -> Result<(), AppError> {
        tracing::info!("Initializing license manager");

        let override_path = if self.config.license_file.is_empty() {
            None
        } else {
            Some(self.config.license_file.as_str())
        };

        self.wrapper.initialize(override_path).map_err(|e| {
            AppError::internal(format!("Failed to initialize license manager: {}", e))
        })?;

        let server_info = self.wrapper.get_server_info();
        let is_star = self.wrapper.is_star_license();
        tracing::info!(
            "License manager initialized: server='{}', star_license={}",
            server_info,
            is_star
        );

        // Initial pool sync
        self.sync_pool_status().await?;

        tracing::info!("License manager ready");
        Ok(())
    }

    /// Shutdown the license system.
    ///
    /// Releases all checkouts via the DLL and updates DB records.
    pub async fn shutdown(&self) -> Result<(), AppError> {
        tracing::info!("Shutting down license manager");

        // Mark all active DB checkouts as checked in
        let active_checkouts =
            self.checkout_repo.find_all_active().await.map_err(|e| {
                AppError::internal(format!("Failed to find active checkouts: {}", e))
            })?;

        let count = active_checkouts.len();

        // Release all via DLL in one call
        self.wrapper.release_all();

        // Update DB records
        for checkout in &active_checkouts {
            if let Err(e) = self.checkout_repo.checkin(checkout.id).await {
                tracing::error!(
                    "Failed to update checkout record {} during shutdown: {}",
                    checkout.id,
                    e
                );
            }
        }

        tracing::info!("License manager shutdown: released {} checkouts", count);
        Ok(())
    }

    /// Checkout a license for a session.
    ///
    /// Called after session creation during the login flow:
    /// `login → create session → checkout(feature, session_id)`
    pub async fn checkout(
        &self,
        user_id: UserId,
        session_id: SessionId,
        ip_address: Option<String>,
    ) -> Result<LicenseCheckout, AppError> {
        let feature = &self.config.feature_name;
        let session_id_str = session_id.to_string();

        tracing::debug!(
            "Checking out license: feature='{}', user={}, session={}",
            feature,
            user_id,
            session_id_str
        );

        // Call DLL checkout
        self.wrapper
            .checkout(feature, &session_id_str)
            .map_err(|e| {
                AppError::service_unavailable(format!("License checkout failed: {}", e))
            })?;

        // Record in database
        let checkout = self
            .checkout_repo
            .create(
                session_id.into_uuid(),
                user_id.into_uuid(),
                feature,
                &session_id_str,
                ip_address.as_deref(),
            )
            .await
            .map_err(|e| {
                // Rollback: checkin the DLL checkout if DB write fails
                tracing::error!(
                    "DB checkout record failed, rolling back DLL checkout: {}",
                    e
                );
                if let Err(rollback_err) = self.wrapper.checkin(feature, &session_id_str) {
                    tracing::error!("Rollback checkin also failed: {}", rollback_err);
                }
                AppError::internal(format!("Failed to save checkout record: {}", e))
            })?;

        self.invalidate_cache().await;

        tracing::info!(
            "License checked out: feature='{}', session='{}', user={}",
            feature,
            session_id_str,
            user_id
        );

        Ok(checkout)
    }

    /// Checkin (release) a license for a session.
    ///
    /// Called during logout: `checkin(feature, session_id) → destroy session`
    pub async fn checkin_by_session(&self, session_id: SessionId) -> Result<(), AppError> {
        let feature = &self.config.feature_name;
        let session_id_str = session_id.to_string();

        tracing::debug!(
            "Checking in license: feature='{}', session='{}'",
            feature,
            session_id_str
        );

        // Call DLL checkin
        if let Err(e) = self.wrapper.checkin(feature, &session_id_str) {
            tracing::warn!(
                "DLL checkin warning for session '{}': {} — continuing with DB cleanup",
                session_id_str,
                e
            );
        }

        // Update DB: mark checkout as checked in
        let active_checkouts = self
            .checkout_repo
            .find_active_by_session(session_id.into_uuid())
            .await
            .map_err(|e| AppError::internal(format!("Failed to find session checkouts: {}", e)))?;

        for checkout in &active_checkouts {
            if let Err(e) = self.checkout_repo.checkin(checkout.id).await {
                tracing::error!("Failed to update checkout record {}: {}", checkout.id, e);
            }
        }

        if !active_checkouts.is_empty() {
            self.invalidate_cache().await;
        }

        tracing::info!(
            "License checked in: feature='{}', session='{}'",
            feature,
            session_id_str
        );

        Ok(())
    }

    /// Get the current pool status.
    ///
    /// Returns cached status if within TTL, otherwise queries the DLL.
    pub async fn pool_status(&self) -> Result<PoolStatus, AppError> {
        let cache_ttl = std::time::Duration::from_secs(self.config.pool.cache_ttl_seconds);

        {
            let cached = self.cached_status.read().await;
            if let Some(ref status) = *cached {
                let age = Utc::now() - status.cached_at;
                if age.to_std().unwrap_or(std::time::Duration::MAX) < cache_ttl {
                    return Ok(status.status.clone());
                }
            }
        }

        self.sync_pool_status().await
    }

    /// Force sync pool status from the DLL.
    pub async fn sync_pool_status(&self) -> Result<PoolStatus, AppError> {
        let feature = &self.config.feature_name;

        let (total_seats, used_seats) = self
            .wrapper
            .get_token_pool(feature)
            .map_err(|e| AppError::internal(format!("Failed to get token pool: {}", e)))?;

        let is_star = self.wrapper.is_star_license();
        let available = if is_star {
            i32::MAX
        } else {
            total_seats - used_seats
        };

        let active_db_sessions =
            self.checkout_repo.count_active().await.map_err(|e| {
                AppError::internal(format!("Failed to count active checkouts: {}", e))
            })?;

        let admin_reserved = if self.config.pool.admin_reserved_enabled {
            self.config.pool.admin_reserved_seats as i32
        } else {
            0
        };

        let drift_detected = !is_star && (used_seats - active_db_sessions as i32).abs() > 0;
        let drift_detail = if drift_detected {
            Some(serde_json::json!({
                "dll_used": used_seats,
                "db_active": active_db_sessions,
                "difference": used_seats - active_db_sessions as i32,
            }))
        } else {
            None
        };

        let usage_percent = if is_star || total_seats == 0 {
            0.0
        } else {
            (used_seats as f64 / total_seats as f64) * 100.0
        };

        let status = PoolStatus {
            total_seats: if is_star { -1 } else { total_seats },
            checked_out: used_seats,
            available,
            admin_reserved,
            active_sessions: active_db_sessions as i32,
            drift_detected,
            usage_percent,
        };

        // Save snapshot
        let _ = self
            .snapshot_repo
            .create(
                status.total_seats,
                status.checked_out,
                status.available,
                status.admin_reserved,
                status.active_sessions,
                status.drift_detected,
                drift_detail.as_ref(),
                "sync",
            )
            .await;

        // Update cache
        {
            let mut cached = self.cached_status.write().await;
            *cached = Some(CachedPoolStatus {
                status: status.clone(),
                cached_at: Utc::now(),
            });
        }

        Ok(status)
    }

    /// Reconcile pool state — fix drift between DLL and database.
    ///
    /// Finds DB checkouts that don't correspond to DLL state and cleans them up.
    pub async fn reconcile(&self) -> Result<PoolStatus, AppError> {
        tracing::info!("Starting pool reconciliation");

        let feature = &self.config.feature_name;

        // Get DLL state
        let (_, used) = self
            .wrapper
            .get_token_pool(feature)
            .map_err(|e| AppError::internal(format!("Failed to get DLL pool state: {}", e)))?;

        // Get DB state
        let active_checkouts =
            self.checkout_repo.find_all_active().await.map_err(|e| {
                AppError::internal(format!("Failed to find active checkouts: {}", e))
            })?;

        let db_count = active_checkouts.len() as i32;
        let drift = used - db_count;

        if drift != 0 {
            tracing::warn!(
                "Pool drift detected: DLL reports {} used, DB has {} active (drift: {})",
                used,
                db_count,
                drift
            );

            if drift < 0 {
                // DB has more checkouts than DLL — orphaned DB records
                // These sessions may have been released by the DLL without DB update
                tracing::info!(
                    "Found {} orphaned DB checkout records, cleaning up",
                    drift.abs()
                );

                // We can't easily tell which DB records are orphaned without
                // querying the DLL per-session, so we re-checkout all DB sessions
                // to ensure consistency
                for checkout in &active_checkouts {
                    let session_id_str = checkout
                        .session_id
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| checkout.checkout_token.clone());

                    if self.wrapper.checkout(feature, &session_id_str).is_err() {
                        tracing::warn!(
                            "Reconciliation: checkout for session '{}' failed, marking as checked in",
                            session_id_str
                        );
                        if let Err(e) = self.checkout_repo.checkin(checkout.id).await {
                            tracing::error!(
                                "Failed to mark orphaned checkout {}: {}",
                                checkout.id,
                                e
                            );
                        }
                    }
                }
            }
        } else {
            tracing::info!(
                "Pool reconciliation: no drift detected (DLL={}, DB={})",
                used,
                db_count
            );
        }

        // Force fresh sync
        let status = self.sync_pool_status().await?;
        tracing::info!("Pool reconciliation completed: {:?}", status);
        Ok(status)
    }

    /// Get pool snapshot history
    pub async fn pool_history(&self, limit: i64) -> Result<Vec<PoolSnapshot>, AppError> {
        use filehub_core::types::pagination::PageRequest;
        let page = PageRequest::new(1, limit as u64);
        let response = self
            .snapshot_repo
            .find_recent(&page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get pool history: {}", e)))?;

        Ok(response.items)
    }

    /// Check if the license manager is using mock implementation
    pub fn is_mock(&self) -> bool {
        self.wrapper.is_mock()
    }

    /// Check if this is a star (unlimited) license
    pub fn is_star_license(&self) -> bool {
        self.wrapper.is_star_license()
    }

    /// Get the license server info string
    pub fn server_info(&self) -> String {
        self.wrapper.get_server_info()
    }

    /// Get the feature name from config
    pub fn feature_name(&self) -> &str {
        &self.config.feature_name
    }

    /// Release all licenses (emergency shutdown)
    pub fn release_all(&self) {
        self.wrapper.release_all();
    }

    /// Invalidate the cached pool status
    pub async fn invalidate_cache(&self) {
        self.cached_status.write().await.take();
    }

    /// Get the critical threshold percentage from config
    pub fn critical_threshold_percent(&self) -> u8 {
        self.config.pool.critical_threshold_percent
    }
}
