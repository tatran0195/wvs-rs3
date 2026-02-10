//! Hook implementations for the FlexNet plugin.
//!
//! Implements all lifecycle hooks that integrate license management
//! with the FileHub session system.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::{SessionId, UserId};
use filehub_plugin::hooks::definitions::{HookAction, HookContext, HookPayload, HookResult};

use crate::license::checkin::CheckinReason;
use crate::license::manager::LicenseManager;

/// Hook handler for after_login: checkout a license
pub struct AfterLoginHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl AfterLoginHook {
    /// Create a new after_login hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl filehub_plugin::hooks::definitions::HookHandler for AfterLoginHook {
    fn name(&self) -> &str {
        "flexnet_after_login"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, payload: &HookPayload) -> HookResult {
        let user_id = ctx
            .user_id
            .ok_or_else(|| AppError::internal("after_login hook: user_id not set in context"))?;
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("after_login hook: session_id not set in context"))?;

        let ip_address = ctx.ip_address.clone();

        tracing::info!(
            "FlexNet after_login: checking out license for user={}, session={}",
            user_id,
            session_id
        );

        match self
            .manager
            .checkout(
                UserId::from(user_id),
                SessionId::from(session_id),
                ip_address,
            )
            .await
        {
            Ok(checkout) => {
                tracing::info!(
                    "License checked out successfully: token='{}'",
                    checkout.checkout_token
                );
                Ok(HookAction::Continue(Some(serde_json::json!({
                    "checkout_token": checkout.checkout_token,
                    "feature": checkout.feature_name,
                }))))
            }
            Err(e) => {
                tracing::error!("License checkout failed: {}", e);
                Ok(HookAction::Halt(format!("License checkout failed: {}", e)))
            }
        }
    }
}

/// Hook handler for before_logout: prepare checkin
pub struct BeforeLogoutHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl BeforeLogoutHook {
    /// Create a new before_logout hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl filehub_plugin::hooks::definitions::HookHandler for BeforeLogoutHook {
    fn name(&self) -> &str {
        "flexnet_before_logout"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx.session_id.ok_or_else(|| {
            AppError::internal("before_logout hook: session_id not set in context")
        })?;

        tracing::info!(
            "FlexNet before_logout: preparing checkin for session={}",
            session_id
        );

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            tracing::error!(
                "Failed to checkin license during logout for session={}: {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook handler for after_session_terminate: checkin license
pub struct AfterSessionTerminateHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl AfterSessionTerminateHook {
    /// Create a new after_session_terminate hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl filehub_plugin::hooks::definitions::HookHandler for AfterSessionTerminateHook {
    fn name(&self) -> &str {
        "flexnet_after_session_terminate"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx.session_id.ok_or_else(|| {
            AppError::internal("after_session_terminate hook: session_id not set")
        })?;

        tracing::info!(
            "FlexNet after_session_terminate: checking in license for session={}",
            session_id
        );

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            tracing::error!(
                "Failed to checkin license after session termination for session={}: {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook handler for on_session_idle: consider releasing license
pub struct OnSessionIdleHook {
    /// License manager
    manager: Arc<LicenseManager>,
    /// Whether to release licenses on idle
    release_on_idle: bool,
}

impl OnSessionIdleHook {
    /// Create a new on_session_idle hook handler
    pub fn new(manager: Arc<LicenseManager>, release_on_idle: bool) -> Self {
        Self {
            manager,
            release_on_idle,
        }
    }
}

#[async_trait]
impl filehub_plugin::hooks::definitions::HookHandler for OnSessionIdleHook {
    fn name(&self) -> &str {
        "flexnet_on_session_idle"
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        if !self.release_on_idle {
            return Ok(HookAction::Continue(None));
        }

        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("on_session_idle hook: session_id not set"))?;

        tracing::info!(
            "FlexNet on_session_idle: considering license release for session={}",
            session_id
        );

        let pool_status = self.manager.pool_status().await?;
        let utilization = if pool_status.total_seats > 0 {
            (pool_status.checked_out as f64 / pool_status.total_seats as f64) * 100.0
        } else {
            0.0
        };

        if utilization >= pool_status.critical_threshold as f64 {
            tracing::info!(
                "Pool utilization at {:.1}% (critical threshold: {}%), releasing idle license",
                utilization,
                pool_status.critical_threshold
            );

            if let Err(e) = self
                .manager
                .checkin_by_session(SessionId::from(session_id))
                .await
            {
                tracing::error!(
                    "Failed to release idle license for session={}: {}",
                    session_id,
                    e
                );
            }
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook handler for on_session_expired: checkin license
pub struct OnSessionExpiredHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl OnSessionExpiredHook {
    /// Create a new on_session_expired hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl filehub_plugin::hooks::definitions::HookHandler for OnSessionExpiredHook {
    fn name(&self) -> &str {
        "flexnet_on_session_expired"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("on_session_expired hook: session_id not set"))?;

        tracing::info!(
            "FlexNet on_session_expired: checking in license for session={}",
            session_id
        );

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            tracing::error!(
                "Failed to checkin license on session expiry for session={}: {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}
