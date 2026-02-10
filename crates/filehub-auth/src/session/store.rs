//! Session storage operations wrapping the database repository.

use std::net::IpAddr;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use filehub_core::config::SessionConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::session::SessionRepository;
use filehub_entity::session::{PresenceStatus, Session};

/// Abstracts session persistence operations.
#[derive(Debug, Clone)]
pub struct SessionStore {
    /// Session database repository.
    repo: Arc<SessionRepository>,
    /// Session configuration.
    config: SessionConfig,
}

impl SessionStore {
    /// Creates a new session store.
    pub fn new(repo: Arc<SessionRepository>, config: SessionConfig) -> Self {
        Self { repo, config }
    }

    /// Creates a new session record in the database.
    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        refresh_token_hash: &str,
        ip_address: IpAddr,
        user_agent: Option<&str>,
        device_info: Option<serde_json::Value>,
    ) -> Result<Session, AppError> {
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.config.absolute_timeout_hours as i64);

        let session = Session {
            id: Uuid::new_v4(),
            user_id,
            token_hash: token_hash.to_string(),
            refresh_token_hash: Some(refresh_token_hash.to_string()),
            ip_address: ip_address.to_string(),
            user_agent: user_agent.map(String::from),
            device_info,
            license_checkout_id: None,
            seat_allocated_at: None,
            overflow_kicked: None,
            presence_status: PresenceStatus::Active,
            ws_connected: false,
            ws_connected_at: None,
            terminated_by: None,
            terminated_reason: None,
            terminated_at: None,
            created_at: now,
            expires_at,
            last_activity: now,
        };

        self.repo
            .create(&session)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create session: {e}")))?;

        Ok(session)
    }

    /// Finds a session by ID.
    pub async fn find_by_id(&self, session_id: Uuid) -> Result<Option<Session>, AppError> {
        self.repo
            .find_by_id(session_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to find session: {e}")))
    }

    /// Counts active (non-terminated, non-expired) sessions for a user.
    pub async fn count_active_by_user(&self, user_id: Uuid) -> Result<i64, AppError> {
        self.repo
            .count_active_by_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count active sessions: {e}")))
    }

    /// Finds the oldest active session for a user (for kick_oldest strategy).
    pub async fn find_oldest_by_user(&self, user_id: Uuid) -> Result<Option<Session>, AppError> {
        self.repo
            .find_oldest_active_by_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to find oldest session: {e}")))
    }

    /// Finds the most idle active session for a user (for kick_idle strategy).
    pub async fn find_most_idle_by_user(&self, user_id: Uuid) -> Result<Option<Session>, AppError> {
        self.repo
            .find_most_idle_by_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to find most idle session: {e}")))
    }

    /// Finds all active sessions for a user.
    pub async fn find_active_by_user(&self, user_id: Uuid) -> Result<Vec<Session>, AppError> {
        self.repo
            .find_active_by_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to find active sessions: {e}")))
    }

    /// Updates session's last activity timestamp.
    pub async fn touch_activity(&self, session_id: Uuid) -> Result<(), AppError> {
        self.repo
            .update_last_activity(session_id, Utc::now())
            .await
            .map_err(|e| AppError::internal(format!("Failed to update activity: {e}")))
    }

    /// Updates session license checkout info.
    pub async fn set_license_checkout(
        &self,
        session_id: Uuid,
        checkout_id: &str,
    ) -> Result<(), AppError> {
        self.repo
            .set_license_checkout(session_id, checkout_id, Utc::now())
            .await
            .map_err(|e| AppError::internal(format!("Failed to set license checkout: {e}")))
    }

    /// Sets the seat allocation timestamp.
    pub async fn set_seat_allocated(&self, session_id: Uuid) -> Result<(), AppError> {
        self.repo
            .set_seat_allocated(session_id, Utc::now())
            .await
            .map_err(|e| AppError::internal(format!("Failed to set seat allocation: {e}")))
    }

    /// Marks a session as terminated.
    pub async fn terminate_session(
        &self,
        session_id: Uuid,
        terminated_by: Option<Uuid>,
        reason: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        self.repo
            .terminate(session_id, terminated_by, reason, now)
            .await
            .map_err(|e| AppError::internal(format!("Failed to terminate session: {e}")))
    }

    /// Deletes a session record.
    pub async fn delete_session(&self, session_id: Uuid) -> Result<(), AppError> {
        self.repo
            .delete(session_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete session: {e}")))
    }

    /// Finds all expired or idle sessions eligible for cleanup.
    pub async fn find_expired_sessions(&self) -> Result<Vec<Session>, AppError> {
        let now = Utc::now();
        let idle_cutoff = now - Duration::minutes(self.config.idle_timeout_minutes as i64);

        self.repo
            .find_expired_or_idle(now, idle_cutoff)
            .await
            .map_err(|e| AppError::internal(format!("Failed to find expired sessions: {e}")))
    }

    /// Counts total active sessions across all users.
    pub async fn count_all_active(&self) -> Result<i64, AppError> {
        self.repo
            .count_all_active()
            .await
            .map_err(|e| AppError::internal(format!("Failed to count all active sessions: {e}")))
    }

    /// Finds all active sessions (for admin view).
    pub async fn find_all_active(&self) -> Result<Vec<Session>, AppError> {
        self.repo
            .find_all_active()
            .await
            .map_err(|e| AppError::internal(format!("Failed to find all active sessions: {e}")))
    }

    /// Updates WebSocket connection state.
    pub async fn set_ws_connected(
        &self,
        session_id: Uuid,
        connected: bool,
    ) -> Result<(), AppError> {
        let connected_at = if connected { Some(Utc::now()) } else { None };
        self.repo
            .set_ws_connected(session_id, connected, connected_at)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update WS state: {e}")))
    }

    /// Updates session presence status.
    pub async fn set_presence_status(
        &self,
        session_id: Uuid,
        status: PresenceStatus,
    ) -> Result<(), AppError> {
        self.repo
            .set_presence_status(session_id, status)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update presence: {e}")))
    }

    /// Updates the refresh token hash (on token rotation).
    pub async fn update_refresh_token(
        &self,
        session_id: Uuid,
        new_hash: &str,
    ) -> Result<(), AppError> {
        self.repo
            .update_refresh_token_hash(session_id, new_hash)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update refresh token: {e}")))
    }
}
