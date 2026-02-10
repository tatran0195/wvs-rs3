//! Session limit resolution — determines the maximum concurrent sessions for a user.
//!
//! Resolution order:
//! 1. Per-user override (from `user_session_limits` table)
//! 2. Per-role configuration (from config)
//! 3. Default (unlimited / pool-bounded)

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::config::SessionConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::session::SessionLimitRepository;
use filehub_entity::user::UserRole;

/// Resolves session limits for individual users based on overrides, role config, and defaults.
#[derive(Debug, Clone)]
pub struct SessionLimiter {
    /// Repository for per-user session limit overrides.
    limit_repo: Arc<SessionLimitRepository>,
    /// Session configuration with per-role limits.
    config: SessionConfig,
}

impl SessionLimiter {
    /// Creates a new session limiter.
    pub fn new(limit_repo: Arc<SessionLimitRepository>, config: SessionConfig) -> Self {
        Self { limit_repo, config }
    }

    /// Resolves the effective maximum concurrent sessions for a user.
    ///
    /// Returns `None` if unlimited (pool-bounded only), or `Some(max)`.
    ///
    /// Resolution order:
    /// 1. Per-user override from database
    /// 2. Per-role limit from configuration
    /// 3. None (unlimited)
    pub async fn resolve_limit(
        &self,
        user_id: Uuid,
        role: &UserRole,
    ) -> Result<Option<u32>, AppError> {
        // 1. Check per-user override
        if let Some(user_limit) = self
            .limit_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to query user session limit: {e}")))?
        {
            return Ok(Some(user_limit.max_sessions as u32));
        }

        // 2. Check per-role configuration
        let role_limit = self.get_role_limit(role);

        if role_limit == 0 {
            // 0 means unlimited (pool-bounded)
            return Ok(None);
        }

        Ok(Some(role_limit))
    }

    /// Gets the configured limit for a specific role.
    fn get_role_limit(&self, role: &UserRole) -> u32 {
        match role {
            UserRole::Admin => self.config.limits.by_role.admin,
            UserRole::Manager => self.config.limits.by_role.manager,
            UserRole::Creator => self.config.limits.by_role.creator,
            UserRole::Viewer => self.config.limits.by_role.viewer,
        }
    }

    /// Checks whether session limits are enabled in configuration.
    pub fn limits_enabled(&self) -> bool {
        self.config.limits.enabled
    }

    /// Returns the configured overflow strategy.
    pub fn overflow_strategy(&self) -> &str {
        &self.config.limits.overflow_strategy
    }

    /// Sets a per-user session limit override.
    pub async fn set_user_limit(
        &self,
        user_id: Uuid,
        max_sessions: u32,
        reason: Option<&str>,
        set_by: Uuid,
    ) -> Result<(), AppError> {
        self.limit_repo
            .upsert(user_id, max_sessions as i32, reason, set_by)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set user session limit: {e}")))
    }

    /// Removes a per-user session limit override (falls back to role default).
    pub async fn remove_user_limit(&self, user_id: Uuid) -> Result<(), AppError> {
        self.limit_repo
            .delete(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to remove user session limit: {e}")))
    }

    /// Gets the current per-role limit configuration.
    pub fn role_limits(&self) -> &filehub_core::config::RoleLimits {
        &self.config.limits.by_role
    }
}
