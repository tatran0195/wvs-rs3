//! Session termination — single, bulk, and terminate-all flows.

use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_auth::session::SessionManager;
use filehub_core::error::AppError;

use crate::context::RequestContext;

/// Handles admin session termination operations.
#[derive(Clone)]
pub struct TerminationService {
    /// Session manager for termination.
    session_manager: Arc<SessionManager>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

impl std::fmt::Debug for TerminationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminationService").finish()
    }
}

/// Request for bulk session termination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BulkTerminateRequest {
    /// List of session IDs to terminate.
    pub session_ids: Vec<Uuid>,
    /// Reason for termination.
    pub reason: String,
}

impl TerminationService {
    /// Creates a new termination service.
    pub fn new(session_manager: Arc<SessionManager>, rbac: Arc<RbacEnforcer>) -> Self {
        Self {
            session_manager,
            rbac,
        }
    }

    /// Terminates a single session (admin).
    pub async fn terminate_session(
        &self,
        ctx: &RequestContext,
        session_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionTerminate)?;

        self.session_manager
            .admin_terminate(session_id, ctx.user_id, reason)
            .await?;

        info!(
            admin_id = %ctx.user_id,
            session_id = %session_id,
            reason = %reason,
            "Session terminated by admin"
        );

        Ok(())
    }

    /// Terminates multiple sessions in bulk.
    pub async fn bulk_terminate(
        &self,
        ctx: &RequestContext,
        req: BulkTerminateRequest,
    ) -> Result<u32, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionTerminate)?;

        let mut terminated = 0u32;

        for session_id in &req.session_ids {
            match self
                .session_manager
                .admin_terminate(*session_id, ctx.user_id, &req.reason)
                .await
            {
                Ok(()) => terminated += 1,
                Err(e) => {
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to terminate session in bulk"
                    );
                }
            }
        }

        info!(
            admin_id = %ctx.user_id,
            requested = req.session_ids.len(),
            terminated = terminated,
            "Bulk termination completed"
        );

        Ok(terminated)
    }

    /// Terminates all non-admin sessions.
    pub async fn terminate_all_non_admin(
        &self,
        ctx: &RequestContext,
        reason: &str,
    ) -> Result<u32, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionTerminate)?;

        let terminated = self
            .session_manager
            .terminate_all_non_admin(ctx.user_id, reason)
            .await?;

        info!(
            admin_id = %ctx.user_id,
            terminated = terminated,
            "All non-admin sessions terminated"
        );

        Ok(terminated)
    }
}
