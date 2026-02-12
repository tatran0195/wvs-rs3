//! Presence reconciliation job — syncs WebSocket presence with database state.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use tracing;

use filehub_database::repositories::session::SessionRepository;
use filehub_entity::job::model::Job;
use filehub_entity::presence::PresenceStatus;

use crate::executor::{JobExecutionError, JobHandler};

/// Trait for presence tracking operations — decouples from filehub-realtime
#[async_trait]
pub trait PresenceService: Send + Sync + std::fmt::Debug {
    /// Get the list of currently connected user IDs
    async fn connected_user_ids(&self) -> Result<Vec<uuid::Uuid>, filehub_core::error::AppError>;

    /// Mark a session as disconnected
    async fn mark_disconnected(
        &self,
        session_id: uuid::Uuid,
    ) -> Result<(), filehub_core::error::AppError>;
}

/// Handles presence reconciliation
#[derive(Debug)]
pub struct PresenceJobHandler {
    /// Session repository
    session_repo: Arc<SessionRepository>,
    /// Optional presence service
    presence_service: Option<Arc<dyn PresenceService>>,
    /// Heartbeat timeout in seconds
    heartbeat_timeout_seconds: i64,
}

impl PresenceJobHandler {
    /// Create a new presence job handler
    pub fn new(
        session_repo: Arc<SessionRepository>,
        presence_service: Option<Arc<dyn PresenceService>>,
        heartbeat_timeout_seconds: i64,
    ) -> Self {
        Self {
            session_repo,
            presence_service,
            heartbeat_timeout_seconds,
        }
    }

    /// Reconcile presence state
    async fn reconcile_presence(&self) -> Result<Value, JobExecutionError> {
        tracing::debug!("Running presence reconciliation");

        let timeout_cutoff = Utc::now() - Duration::seconds(self.heartbeat_timeout_seconds);

        let stale_sessions = self
            .session_repo
            .find_stale_ws_connections(timeout_cutoff)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Failed to find stale connections: {}", e))
            })?;

        let mut disconnected = 0;

        for session in &stale_sessions {
            tracing::debug!(
                "Marking stale WS connection as disconnected: session={}",
                session.id
            );

            if let Err(e) = self.session_repo.update_ws_state(session.id, false).await {
                tracing::warn!(
                    "Failed to update ws_connected for session {}: {}",
                    session.id,
                    e
                );
                continue;
            }

            if let Some(ref presence) = self.presence_service {
                if let Err(e) = presence.mark_disconnected(session.id).await {
                    tracing::warn!(
                        "Failed to mark session {} as disconnected in presence service: {}",
                        session.id,
                        e
                    );
                }
            }

            disconnected += 1;
        }

        if disconnected > 0 {
            tracing::info!(
                "Presence reconciliation: {} stale connections marked as disconnected",
                disconnected
            );
        }

        Ok(serde_json::json!({
            "task": "presence_reconciliation",
            "stale_sessions_found": stale_sessions.len(),
            "disconnected": disconnected,
            "heartbeat_timeout_seconds": self.heartbeat_timeout_seconds,
        }))
    }
}

#[async_trait]
impl JobHandler for PresenceJobHandler {
    fn job_type(&self) -> &str {
        "presence_reconciliation"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let result = self.reconcile_presence().await?;
        Ok(Some(result))
    }
}

/// Handler for idle_session_check job type
#[derive(Debug)]
pub struct IdleSessionCheckHandler {
    /// Session repository
    session_repo: Arc<SessionRepository>,
    /// Idle timeout in minutes
    idle_timeout_minutes: i64,
}

impl IdleSessionCheckHandler {
    /// Create a new idle session check handler
    pub fn new(session_repo: Arc<SessionRepository>, idle_timeout_minutes: i64) -> Self {
        Self {
            session_repo,
            idle_timeout_minutes,
        }
    }
}

#[async_trait]
impl JobHandler for IdleSessionCheckHandler {
    fn job_type(&self) -> &str {
        "idle_session_check"
    }

    async fn execute(&self, _job: &Job) -> Result<Option<Value>, JobExecutionError> {
        tracing::debug!("Running idle session check");

        let idle_cutoff = Utc::now() - Duration::minutes(self.idle_timeout_minutes);

        let idle_sessions = self
            .session_repo
            .find_idle_sessions(idle_cutoff)
            .await
            .map_err(|e| {
                JobExecutionError::Transient(format!("Failed to find idle sessions: {}", e))
            })?;

        let mut marked_idle = 0;

        for session in &idle_sessions {
            if let Err(e) = self
                .session_repo
                .update_presence(session.id, &PresenceStatus::Idle)
                .await
            {
                tracing::warn!("Failed to mark session {} as idle: {}", session.id, e);
                continue;
            }
            marked_idle += 1;
        }

        if marked_idle > 0 {
            tracing::info!(
                "Idle session check: {} sessions marked as idle (timeout={}min)",
                marked_idle,
                self.idle_timeout_minutes
            );
        }

        Ok(Some(serde_json::json!({
            "task": "idle_session_check",
            "idle_sessions_found": idle_sessions.len(),
            "marked_idle": marked_idle,
            "idle_timeout_minutes": self.idle_timeout_minutes,
        })))
    }
}
