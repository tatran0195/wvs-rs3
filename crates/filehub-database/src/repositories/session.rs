//! Session repository implementation.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::session::model::{CreateSession, Session};

/// Repository for session CRUD and query operations.
#[derive(Debug, Clone)]
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    /// Create a new session repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a session by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Session>> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find session", e))
    }

    /// Find a session by token hash.
    pub async fn find_by_token_hash(&self, token_hash: &str) -> AppResult<Option<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE token_hash = $1 AND terminated_at IS NULL",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find session by token", e)
        })
    }

    /// Find a session by refresh token hash.
    pub async fn find_by_refresh_token_hash(&self, hash: &str) -> AppResult<Option<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE refresh_token_hash = $1 AND terminated_at IS NULL",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                "Failed to find session by refresh token",
                e,
            )
        })
    }

    /// List all active sessions for a user.
    pub async fn find_active_by_user(&self, user_id: Uuid) -> AppResult<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE user_id = $1 AND terminated_at IS NULL AND expires_at > NOW() \
             ORDER BY created_at DESC"
        )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find active sessions", e))
    }

    /// Count active sessions for a user.
    pub async fn count_active_by_user(&self, user_id: Uuid) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND terminated_at IS NULL AND expires_at > NOW()"
        )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count active sessions", e))?;
        Ok(count)
    }

    /// Count all active sessions system-wide.
    pub async fn count_all_active(&self) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE terminated_at IS NULL AND expires_at > NOW()",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                "Failed to count all active sessions",
                e,
            )
        })?;
        Ok(count)
    }

    /// Find the oldest active session for a user.
    pub async fn find_oldest_by_user(&self, user_id: Uuid) -> AppResult<Option<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE user_id = $1 AND terminated_at IS NULL AND expires_at > NOW() \
             ORDER BY created_at ASC LIMIT 1"
        )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find oldest session", e))
    }

    /// Find the most idle active session for a user.
    pub async fn find_most_idle_by_user(&self, user_id: Uuid) -> AppResult<Option<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE user_id = $1 AND terminated_at IS NULL AND expires_at > NOW() \
             ORDER BY last_activity ASC LIMIT 1"
        )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find most idle session", e))
    }

    /// List all active sessions with pagination (admin view).
    pub async fn find_all_active(&self, page: &PageRequest) -> AppResult<PageResponse<Session>> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions WHERE terminated_at IS NULL AND expires_at > NOW()",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count sessions", e))?;

        let sessions = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE terminated_at IS NULL AND expires_at > NOW() \
             ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list sessions", e))?;

        Ok(PageResponse::new(
            sessions,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// List all active sessions without pagination (for admin view).
    pub async fn find_active_by_user_all(&self) -> AppResult<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE terminated_at IS NULL AND expires_at > NOW() \
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to list all active sessions", e)
        })
    }

    /// Create a new session.
    pub async fn create(&self, data: &CreateSession) -> AppResult<Session> {
        sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (user_id, token_hash, refresh_token_hash, ip_address, user_agent, device_info, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
        )
            .bind(data.user_id)
            .bind(&data.token_hash)
            .bind(&data.refresh_token_hash)
            .bind(&data.ip_address)
            .bind(&data.user_agent)
            .bind(&data.device_info)
            .bind(data.expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create session", e))
    }

    /// Update last activity timestamp.
    pub async fn update_last_activity(&self, session_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET last_activity = NOW() WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update last activity", e)
            })?;
        Ok(())
    }

    /// Update WebSocket connection state.
    pub async fn update_ws_state(&self, session_id: Uuid, connected: bool) -> AppResult<()> {
        if connected {
            sqlx::query(
                "UPDATE sessions SET ws_connected = TRUE, ws_connected_at = NOW() WHERE id = $1",
            )
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update WS state", e)
            })?;
        } else {
            sqlx::query("UPDATE sessions SET ws_connected = FALSE WHERE id = $1")
                .bind(session_id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to update WS state", e)
                })?;
        }
        Ok(())
    }

    /// Update presence status.
    pub async fn update_presence(
        &self,
        session_id: Uuid,
        status: &filehub_entity::presence::PresenceStatus,
    ) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET presence_status = $2 WHERE id = $1")
            .bind(session_id)
            .bind(status)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update presence", e)
            })?;
        Ok(())
    }

    /// Set license checkout ID on a session.
    pub async fn set_license_checkout(&self, session_id: Uuid, checkout_id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE sessions SET license_checkout_id = $2, seat_allocated_at = NOW() WHERE id = $1",
        )
        .bind(session_id)
        .bind(checkout_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to set license checkout", e)
        })?;
        Ok(())
    }

    /// Terminate a session.
    pub async fn terminate(
        &self,
        session_id: Uuid,
        terminated_by: Uuid,
        reason: &str,
    ) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE sessions SET terminated_by = $2, terminated_reason = $3, terminated_at = NOW() \
             WHERE id = $1 AND terminated_at IS NULL",
        )
        .bind(session_id)
        .bind(terminated_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to terminate session", e)
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!(
                "Active session {session_id} not found"
            )));
        }
        Ok(())
    }

    /// Terminate all active sessions for a user.
    pub async fn terminate_all_by_user(
        &self,
        user_id: Uuid,
        terminated_by: Uuid,
        reason: &str,
    ) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE sessions SET terminated_by = $2, terminated_reason = $3, terminated_at = NOW() \
             WHERE user_id = $1 AND terminated_at IS NULL AND expires_at > NOW()",
        )
        .bind(user_id)
        .bind(terminated_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to terminate user sessions", e)
        })?;

        Ok(result.rows_affected())
    }

    /// Terminate all non-admin sessions.
    pub async fn terminate_all_non_admin(
        &self,
        terminated_by: Uuid,
        reason: &str,
    ) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE sessions SET terminated_by = $1, terminated_reason = $2, terminated_at = NOW() \
             WHERE terminated_at IS NULL AND expires_at > NOW() \
             AND user_id NOT IN (SELECT id FROM users WHERE role = 'admin')",
        )
        .bind(terminated_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                "Failed to terminate non-admin sessions",
                e,
            )
        })?;

        Ok(result.rows_affected())
    }

    /// Delete expired and terminated sessions older than the given cutoff.
    pub async fn cleanup_expired(&self, before: DateTime<Utc>) -> AppResult<u64> {
        let result = sqlx::query(
            "DELETE FROM sessions WHERE (expires_at < $1) OR (terminated_at IS NOT NULL AND terminated_at < $1)"
        )
            .bind(before)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to cleanup sessions", e))?;

        Ok(result.rows_affected())
    }

    /// Delete a session by ID.
    pub async fn delete(&self, session_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to delete session", e)
            })?;
        Ok(result.rows_affected() > 0)
    }

    /// Find expired or idle sessions for cleanup.
    pub async fn find_expired_or_idle(
        &self,
        now: DateTime<Utc>,
        idle_cutoff: DateTime<Utc>,
    ) -> AppResult<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE (expires_at < $1) OR (last_activity < $2 AND terminated_at IS NULL)"
        )
            .bind(now)
            .bind(idle_cutoff)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find expired sessions", e))
    }

    /// Find sessions with stale WebSocket connections (connected but no activity).
    pub async fn find_stale_ws_connections(
        &self,
        cutoff: DateTime<Utc>,
    ) -> AppResult<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE ws_connected = TRUE AND last_activity < $1 AND terminated_at IS NULL",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find stale WS connections", e))
    }

    /// Find active sessions that have been idle for too long.
    pub async fn find_idle_sessions(&self, cutoff: DateTime<Utc>) -> AppResult<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE last_activity < $1 AND terminated_at IS NULL AND (presence_status != 'idle' OR presence_status IS NULL)",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find idle sessions", e))
    }

    /// Set WebSocket connection state.
    pub async fn set_ws_connected(
        &self,
        session_id: Uuid,
        connected: bool,
        connected_at: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        if connected {
            sqlx::query(
                "UPDATE sessions SET ws_connected = TRUE, ws_connected_at = $2 WHERE id = $1",
            )
            .bind(session_id)
            .bind(connected_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update WS state", e)
            })?;
        } else {
            sqlx::query("UPDATE sessions SET ws_connected = FALSE WHERE id = $1")
                .bind(session_id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to update WS state", e)
                })?;
        }
        Ok(())
    }

    /// Set presence status.
    pub async fn set_presence_status(
        &self,
        session_id: Uuid,
        status: filehub_entity::presence::PresenceStatus,
    ) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET presence_status = $2 WHERE id = $1")
            .bind(session_id)
            .bind(&status)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update presence", e)
            })?;
        Ok(())
    }

    /// Set seat allocated timestamp.
    pub async fn set_seat_allocated(&self, session_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET seat_allocated_at = NOW() WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to set seat allocated", e)
            })?;
        Ok(())
    }

    /// Update refresh token hash.
    pub async fn update_refresh_token_hash(
        &self,
        session_id: Uuid,
        new_hash: &str,
    ) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET refresh_token_hash = $2 WHERE id = $1")
            .bind(session_id)
            .bind(new_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update refresh token", e)
            })?;
        Ok(())
    }
}
