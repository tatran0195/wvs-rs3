//! Hook implementations for the FlexNet plugin.
//!
//! Integrates license checkout/checkin with the FileHub session lifecycle:
//!
//! - `after_login` → checkout license for the new session
//! - `before_logout` → checkin license before session destruction
//! - `after_session_terminate` → checkin license after admin termination
//! - `on_session_expired` → checkin license when session expires
//! - `on_session_idle` → optionally release license under pool pressure

use std::sync::Arc;

use async_trait::async_trait;
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::{SessionId, UserId};
use filehub_plugin::hooks::definitions::{
    HookAction, HookContext, HookHandler, HookPayload, HookResult,
};

use crate::license::manager::LicenseManager;

/// Hook: `after_login` — checkout a license for the newly created session.
///
/// Flow: `login → create session → checkout(feature, session_id)`
///
/// If checkout fails (no seats), returns `HookAction::Halt` which
/// causes the login to fail and the session to be rolled back.
#[derive(Debug)]
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
impl HookHandler for AfterLoginHook {
    fn name(&self) -> &str {
        "flexnet_after_login"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let user_id = ctx
            .user_id
            .ok_or_else(|| AppError::internal("after_login: user_id missing from context"))?;
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("after_login: session_id missing from context"))?;

        let ip_address = ctx.ip_address.clone();

        tracing::info!(
            "FlexNet after_login: checkout for user={}, session={}",
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
                tracing::info!("License checkout successful: session='{}'", session_id);
                Ok(HookAction::Continue(Some(serde_json::json!({
                    "license_checkout_id": checkout.id.to_string(),
                    "feature": checkout.feature_name,
                    "is_star": self.manager.is_star_license(),
                }))))
            }
            Err(e) => {
                tracing::error!("License checkout FAILED: {}", e);
                // Halt the login — session will be rolled back
                Ok(HookAction::Halt(format!(
                    "No license seats available: {}",
                    e
                )))
            }
        }
    }
}

/// Hook: `before_logout` — checkin the license before session destruction.
///
/// Flow: `checkin(feature, session_id) → destroy session`
#[derive(Debug)]
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
impl HookHandler for BeforeLogoutHook {
    fn name(&self) -> &str {
        "flexnet_before_logout"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("before_logout: session_id missing from context"))?;

        tracing::info!("FlexNet before_logout: checkin for session={}", session_id);

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            // Checkin failures are non-fatal for logout
            tracing::error!(
                "License checkin failed during logout (session={}): {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook: `after_session_terminate` — checkin license after admin kills a session.
#[derive(Debug)]
pub struct AfterSessionTerminateHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl AfterSessionTerminateHook {
    /// Create a new hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl HookHandler for AfterSessionTerminateHook {
    fn name(&self) -> &str {
        "flexnet_after_session_terminate"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("after_session_terminate: session_id missing"))?;

        tracing::info!(
            "FlexNet after_session_terminate: checkin for session={}",
            session_id
        );

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            tracing::error!(
                "License checkin failed after session termination (session={}): {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook: `on_session_expired` — checkin license when session naturally expires.
#[derive(Debug)]
pub struct OnSessionExpiredHook {
    /// License manager
    manager: Arc<LicenseManager>,
}

impl OnSessionExpiredHook {
    /// Create a new hook handler
    pub fn new(manager: Arc<LicenseManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl HookHandler for OnSessionExpiredHook {
    fn name(&self) -> &str {
        "flexnet_on_session_expired"
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn execute(&self, ctx: &HookContext, _payload: &HookPayload) -> HookResult {
        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("on_session_expired: session_id missing"))?;

        tracing::info!(
            "FlexNet on_session_expired: checkin for session={}",
            session_id
        );

        if let Err(e) = self
            .manager
            .checkin_by_session(SessionId::from(session_id))
            .await
        {
            tracing::error!(
                "License checkin failed on session expiry (session={}): {}",
                session_id,
                e
            );
        }

        Ok(HookAction::Continue(None))
    }
}

/// Hook: `on_session_idle` — consider releasing license under pool pressure.
///
/// Only releases if pool utilization exceeds the critical threshold.
/// Star licenses never release on idle.
#[derive(Debug)]
pub struct OnSessionIdleHook {
    /// License manager
    manager: Arc<LicenseManager>,
    /// Whether idle release is enabled
    release_on_idle: bool,
}

impl OnSessionIdleHook {
    /// Create a new hook handler
    pub fn new(manager: Arc<LicenseManager>, release_on_idle: bool) -> Self {
        Self {
            manager,
            release_on_idle,
        }
    }
}

#[async_trait]
impl HookHandler for OnSessionIdleHook {
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

        // Star licenses have unlimited seats — no need to release on idle
        if self.manager.is_star_license() {
            return Ok(HookAction::Continue(None));
        }

        let session_id = ctx
            .session_id
            .ok_or_else(|| AppError::internal("on_session_idle: session_id missing"))?;

        let pool_status = self.manager.pool_status().await?;

        // Only release if above critical threshold
        let utilization = if pool_status.total_seats > 0 {
            (pool_status.checked_out as f64 / pool_status.total_seats as f64) * 100.0
        } else {
            0.0
        };

        if utilization >= pool_status.critical_threshold as f64 {
            tracing::info!(
                "Pool at {:.1}% utilization (critical: {}%), releasing idle session={}",
                utilization,
                pool_status.critical_threshold,
                session_id
            );

            if let Err(e) = self
                .manager
                .checkin_by_session(SessionId::from(session_id))
                .await
            {
                tracing::error!(
                    "Failed to release idle license (session={}): {}",
                    session_id,
                    e
                );
            }
        }

        Ok(HookAction::Continue(None))
    }
}
