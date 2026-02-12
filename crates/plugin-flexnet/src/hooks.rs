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
use filehub_plugin::prelude::*;

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
impl SimpleHookHandler for AfterLoginHook {
    fn plugin_id(&self) -> &str {
        "flexnet"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::AfterLogin
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        let user_id = payload
            .get_uuid("user_id")
            .ok_or_else(|| AppError::internal("after_login: user_id missing from payload"));
        let session_id = payload
            .get_uuid("session_id")
            .ok_or_else(|| AppError::internal("after_login: session_id missing from payload"));

        let (user_id, session_id) = match (user_id, session_id) {
            (Ok(u), Ok(s)) => (u, s),
            (Err(e), _) | (_, Err(e)) => {
                tracing::error!("FlexNet after_login: {}", e);
                return HookResult::halt("flexnet", &e.to_string());
            }
        };

        let ip_address = payload.get_string("ip_address").map(|s| s.to_string());

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
                HookResult::continue_with_output(
                    "flexnet",
                    serde_json::json!({
                        "license_checkout_id": checkout.id.to_string(),
                        "feature": checkout.feature_name,
                        "is_star": self.manager.is_star_license(),
                    }),
                )
            }
            Err(e) => {
                tracing::error!("License checkout FAILED: {}", e);
                // Halt the login — session will be rolled back
                HookResult::halt("flexnet", &format!("No license seats available: {}", e))
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
impl SimpleHookHandler for BeforeLogoutHook {
    fn plugin_id(&self) -> &str {
        "flexnet"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::BeforeLogout
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        let session_id = match payload.get_uuid("session_id") {
            Some(id) => id,
            None => {
                tracing::error!("FlexNet before_logout: session_id missing from payload");
                return HookResult::continue_execution("flexnet");
            }
        };

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

        HookResult::continue_execution("flexnet")
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
impl SimpleHookHandler for AfterSessionTerminateHook {
    fn plugin_id(&self) -> &str {
        "flexnet"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::AfterSessionTerminate
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        let session_id = match payload.get_uuid("session_id") {
            Some(id) => id,
            None => {
                tracing::error!("FlexNet after_session_terminate: session_id missing from payload");
                return HookResult::continue_execution("flexnet");
            }
        };

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

        HookResult::continue_execution("flexnet")
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
impl SimpleHookHandler for OnSessionExpiredHook {
    fn plugin_id(&self) -> &str {
        "flexnet"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::OnSessionExpired
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        let session_id = match payload.get_uuid("session_id") {
            Some(id) => id,
            None => {
                tracing::error!("FlexNet on_session_expired: session_id missing from payload");
                return HookResult::continue_execution("flexnet");
            }
        };

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

        HookResult::continue_execution("flexnet")
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
impl SimpleHookHandler for OnSessionIdleHook {
    fn plugin_id(&self) -> &str {
        "flexnet"
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::OnSessionIdle
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        if !self.release_on_idle {
            return HookResult::continue_execution("flexnet");
        }

        // Star licenses have unlimited seats — no need to release on idle
        if self.manager.is_star_license() {
            return HookResult::continue_execution("flexnet");
        }

        let session_id = match payload.get_uuid("session_id") {
            Some(id) => id,
            None => {
                tracing::error!("FlexNet on_session_idle: session_id missing from payload");
                return HookResult::continue_execution("flexnet");
            }
        };

        let pool_status = match self.manager.pool_status().await {
            Ok(status) => status,
            Err(e) => {
                tracing::error!("Failed to get pool status: {}", e);
                return HookResult::continue_execution("flexnet");
            }
        };

        // Only release if above critical threshold
        // let utilization = if pool_status.total_seats > 0 {
        //     (pool_status.checked_out as f64 / pool_status.total_seats as f64) * 100.0
        // } else {
        //     0.0
        // };

        // if utilization >= pool_status.critical_threshold as f64 {
        //     tracing::info!(
        //         "Pool at {:.1}% utilization (critical: {}%), releasing idle session={}",
        //         utilization,
        //         pool_status.critical_threshold,
        //         session_id
        //     );

        //     if let Err(e) = self
        //         .manager
        //         .checkin_by_session(SessionId::from(session_id))
        //         .await
        //     {
        //         tracing::error!(
        //             "Failed to release idle license (session={}): {}",
        //             session_id,
        //             e
        //         );
        //     }
        // }

        HookResult::continue_execution("flexnet")
    }
}
