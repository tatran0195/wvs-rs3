//! Session limit repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_entity::session::limit::UserSessionLimit;

/// Repository for user session limit CRUD operations.
#[derive(Debug, Clone)]
pub struct SessionLimitRepository {
    pool: PgPool,
}

impl SessionLimitRepository {
    /// Create a new session limit repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a session limit override for a specific user.
    pub async fn find_by_user_id(&self, user_id: Uuid) -> AppResult<Option<UserSessionLimit>> {
        sqlx::query_as::<_, UserSessionLimit>(
            "SELECT * FROM user_session_limits WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find user session limit", e)
        })
    }

    /// Upsert a session limit override for a user.
    pub async fn upsert(
        &self,
        user_id: Uuid,
        max_sessions: i32,
        reason: Option<&str>,
        set_by: Uuid,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO user_session_limits (user_id, max_sessions, reason, set_by) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id) DO UPDATE SET \
                max_sessions = EXCLUDED.max_sessions, \
                reason = EXCLUDED.reason, \
                set_by = EXCLUDED.set_by, \
                updated_at = NOW()",
        )
        .bind(user_id)
        .bind(max_sessions)
        .bind(reason)
        .bind(set_by)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                "Failed to upsert user session limit",
                e,
            )
        })?;
        Ok(())
    }

    /// Delete a session limit override for a user.
    pub async fn delete(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM user_session_limits WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(
                    ErrorKind::Database,
                    "Failed to delete user session limit",
                    e,
                )
            })?;
        Ok(())
    }

    /// List all session limit overrides.
    pub async fn find_all(&self) -> AppResult<Vec<UserSessionLimit>> {
        sqlx::query_as::<_, UserSessionLimit>(
            "SELECT * FROM user_session_limits ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to list user session limits", e)
        })
    }
}
