//! LicenseManager orchestrates all license operations.

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing;
use uuid::Uuid;

use filehub_core::config::LicenseConfig;
use filehub_core::error::AppError;
use filehub_core::types::id::{SessionId, UserId};
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_entity::license::model::LicenseCheckout;
use filehub_entity::license::pool::{PoolSnapshot, PoolStatus};

use crate::ffi::wrapper::FlexNetWrapper;

/// Manages all FlexNet license operations
#[derive(Debug)]
pub struct LicenseManager {
    /// FlexNet wrapper for FFI calls
    wrapper: Arc<FlexNetWrapper>,
    /// License configuration
    config: LicenseConfig,
    /// License checkout repository
    checkout_repo: Arc<LicenseCheckoutRepository>,
    /// Pool snapshot repository
    snapshot_repo: Arc<PoolSnapshotRepository>,
    /// Cached pool status
    cached_status: Arc<RwLock<Option<CachedPoolStatus>>>,
}

/// Cached pool status with TTL
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
        wrapper: Arc<FlexNetWrapper>,
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

    /// Initialize the license system
    pub async fn initialize(&self) -> Result<(), AppError> {
        tracing::info!("Initializing license manager");
        self.wrapper
            .initialize(&self.config.license_file)
            .map_err(|e| AppError::internal(format!("Failed to initialize FlexNet: {}", e)))?;

        self.sync_pool_status().await?;
        tracing::info!("License manager initialized successfully");
        Ok(())
    }

    /// Shutdown the license system, checking in all active licenses
    pub async fn shutdown(&self) -> Result<(), AppError> {
        tracing::info!("Shutting down license manager");

        let active_checkouts =
            self.checkout_repo.find_active().await.map_err(|e| {
                AppError::internal(format!("Failed to find active checkouts: {}", e))
            })?;

        for checkout in &active_checkouts {
            if let Err(e) = self.wrapper.checkin(&checkout.checkout_token) {
                tracing::error!(
                    "Failed to checkin token '{}' during shutdown: {}",
                    checkout.checkout_token,
                    e
                );
            }
            if let Err(e) = self.checkout_repo.checkin(checkout.id).await {
                tracing::error!(
                    "Failed to update checkout record {} during shutdown: {}",
                    checkout.id,
                    e
                );
            }
        }

        tracing::info!(
            "Checked in {} licenses during shutdown",
            active_checkouts.len()
        );

        self.wrapper
            .shutdown()
            .map_err(|e| AppError::internal(format!("Failed to shutdown FlexNet: {}", e)))?;

        Ok(())
    }

    /// Checkout a license for a session
    pub async fn checkout(
        &self,
        user_id: UserId,
        session_id: SessionId,
        ip_address: Option<String>,
    ) -> Result<LicenseCheckout, AppError> {
        let feature = &self.config.feature_name;
        let version = "1.0";

        tracing::debug!(
            "Checking out license for user={}, session={}, feature='{}'",
            user_id,
            session_id,
            feature
        );

        let result = self.wrapper.checkout(feature, version).map_err(|e| {
            AppError::service_unavailable(format!("License checkout failed: {}", e))
        })?;

        let checkout = LicenseCheckout {
            id: uuid::Uuid::new_v4(),
            session_id: Some(*session_id),
            user_id: *user_id,
            feature_name: feature.clone(),
            checkout_token: result.token,
            checked_out_at: result.checked_out_at,
            checked_in_at: None,
            ip_address,
            is_active: true,
        };

        self.checkout_repo
            .create(&checkout)
            .await
            .map_err(|e| AppError::internal(format!("Failed to save checkout record: {}", e)))?;

        self.invalidate_cache().await;

        tracing::info!(
            "License checked out: user={}, session={}, token='{}'",
            user_id,
            session_id,
            checkout.checkout_token
        );

        Ok(checkout)
    }

    /// Checkin a license for a session
    pub async fn checkin_by_session(&self, session_id: SessionId) -> Result<(), AppError> {
        let checkouts = self
            .checkout_repo
            .find_active_by_session(*session_id)
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to find checkouts for session: {}", e))
            })?;

        for checkout in &checkouts {
            self.checkin_single(checkout).await?;
        }

        if !checkouts.is_empty() {
            self.invalidate_cache().await;
        }

        Ok(())
    }

    /// Checkin a license by its token
    pub async fn checkin_by_token(&self, token: &str) -> Result<(), AppError> {
        self.wrapper
            .checkin(token)
            .map_err(|e| AppError::internal(format!("License checkin failed: {}", e)))?;

        self.checkout_repo
            .checkin_by_token(token)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update checkout record: {}", e)))?;

        self.invalidate_cache().await;
        Ok(())
    }

    /// Get the current pool status
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

    /// Force sync pool status from FlexNet
    pub async fn sync_pool_status(&self) -> Result<PoolStatus, AppError> {
        let feature = &self.config.feature_name;

        let ffi_status = self
            .wrapper
            .pool_status(feature)
            .map_err(|e| AppError::internal(format!("Failed to get pool status: {}", e)))?;

        let active_sessions =
            self.checkout_repo.count_active().await.map_err(|e| {
                AppError::internal(format!("Failed to count active checkouts: {}", e))
            })?;

        let admin_reserved = if self.config.pool.admin_reserved_enabled() {
            self.config.pool.admin_reserved_seats() as i32
        } else {
            0
        };

        let drift_detected = (ffi_status.checked_out_seats - active_sessions as i32).abs() > 0;
        let drift_detail = if drift_detected {
            Some(serde_json::json!({
                "flexnet_checked_out": ffi_status.checked_out_seats,
                "db_active_sessions": active_sessions,
                "difference": ffi_status.checked_out_seats - active_sessions as i32,
            }))
        } else {
            None
        };

        let status = PoolStatus {
            total_seats: ffi_status.total_seats,
            checked_out: ffi_status.checked_out_seats,
            available: ffi_status.available_seats,
            admin_reserved,
            active_sessions: active_sessions as i32,
            warning_threshold: self.config.pool.warning_threshold_percent,
            critical_threshold: self.config.pool.critical_threshold_percent,
            drift_detected,
        };

        let snapshot = PoolSnapshot {
            id: Uuid::new_v4(),
            total_seats: status.total_seats,
            checked_out: status.checked_out,
            available: status.available,
            admin_reserved: status.admin_reserved,
            active_sessions: status.active_sessions,
            drift_detected: status.drift_detected,
            drift_detail,
            source: "sync".to_string(),
            created_at: Utc::now(),
        };

        if let Err(e) = self.snapshot_repo.create(&snapshot).await {
            tracing::warn!("Failed to save pool snapshot: {}", e);
        }

        {
            let mut cached = self.cached_status.write().await;
            *cached = Some(CachedPoolStatus {
                status: status.clone(),
                cached_at: Utc::now(),
            });
        }

        Ok(status)
    }

    /// Reconcile pool status — fix drift between FlexNet and database
    pub async fn reconcile(&self) -> Result<PoolStatus, AppError> {
        tracing::info!("Starting pool reconciliation");

        let active_checkouts =
            self.checkout_repo.find_active().await.map_err(|e| {
                AppError::internal(format!("Failed to find active checkouts: {}", e))
            })?;

        let feature = &self.config.feature_name;
        let ffi_available = self
            .wrapper
            .pool_status(feature)
            .map_err(|e| AppError::internal(format!("Failed to get FFI pool status: {}", e)))?;

        let mut orphan_count = 0;
        for checkout in &active_checkouts {
            if self.wrapper.checkin(&checkout.checkout_token).is_err() {
                tracing::warn!(
                    "Orphaned checkout detected: token='{}', marking as checked in",
                    checkout.checkout_token
                );
                if let Err(e) = self.checkout_repo.checkin(checkout.id).await {
                    tracing::error!("Failed to mark orphaned checkout: {}", e);
                }
                orphan_count += 1;
            } else {
                let re_checkout = self.wrapper.checkout(feature, "1.0");
                match re_checkout {
                    Ok(result) => {
                        if let Err(e) = self
                            .checkout_repo
                            .update_token(checkout.id, &result.token)
                            .await
                        {
                            tracing::error!("Failed to update token during reconciliation: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to re-checkout during reconciliation: {}", e);
                    }
                }
            }
        }

        if orphan_count > 0 {
            tracing::warn!("Reconciliation found {} orphaned checkouts", orphan_count);
        }

        let status = self.sync_pool_status().await?;
        tracing::info!("Pool reconciliation completed: {:?}", status);
        Ok(status)
    }

    /// Get pool snapshot history
    pub async fn pool_history(&self, limit: i64) -> Result<Vec<PoolSnapshot>, AppError> {
        self.snapshot_repo
            .find_recent(limit)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get pool history: {}", e)))
    }

    /// Check if the wrapper is initialized
    pub fn is_initialized(&self) -> bool {
        self.wrapper.is_initialized()
    }

    /// Get the feature name from config
    pub fn feature_name(&self) -> &str {
        &self.config.feature_name
    }

    /// Internal: checkin a single checkout record
    async fn checkin_single(&self, checkout: &LicenseCheckout) -> Result<(), AppError> {
        if let Err(e) = self.wrapper.checkin(&checkout.checkout_token) {
            tracing::warn!(
                "Failed to checkin token '{}' via FFI: {} — marking as checked in anyway",
                checkout.checkout_token,
                e
            );
        }

        self.checkout_repo
            .checkin(checkout.id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update checkout record: {}", e)))?;

        tracing::debug!(
            "License checked in: token='{}', user={}",
            checkout.checkout_token,
            checkout.user_id
        );

        Ok(())
    }

    /// Invalidate the cached pool status
    async fn invalidate_cache(&self) {
        let mut cached = self.cached_status.write().await;
        *cached = None;
    }
}
