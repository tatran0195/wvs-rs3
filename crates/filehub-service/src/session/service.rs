//! Session listing and management for admin views.

use std::sync::Arc;

use uuid::Uuid;

use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_auth::session::SessionStore;
use filehub_core::error::AppError;
use filehub_entity::session::Session;

use crate::context::RequestContext;

/// Admin session viewing and management service.
#[derive(Debug, Clone)]
pub struct SessionService {
    /// Session store.
    session_store: Arc<SessionStore>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

impl SessionService {
    /// Creates a new session service.
    pub fn new(session_store: Arc<SessionStore>, rbac: Arc<RbacEnforcer>) -> Self {
        Self {
            session_store,
            rbac,
        }
    }

    /// Lists all active sessions (admin).
    pub async fn list_active_sessions(
        &self,
        ctx: &RequestContext,
    ) -> Result<Vec<Session>, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionViewAll)?;

        self.session_store.find_all_active().await
    }

    /// Gets details for a specific session (admin).
    pub async fn get_session(
        &self,
        ctx: &RequestContext,
        session_id: Uuid,
    ) -> Result<Session, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionViewAll)?;

        self.session_store
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::not_found("Session not found"))
    }

    /// Gets active session count for a user.
    pub async fn count_user_sessions(
        &self,
        ctx: &RequestContext,
        user_id: Uuid,
    ) -> Result<i64, AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::SessionViewAll)?;

        self.session_store.count_active_by_user(user_id).await
    }
}
