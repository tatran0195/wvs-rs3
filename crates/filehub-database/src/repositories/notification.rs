//! Notification repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::notification::model::Notification;
use filehub_entity::notification::preference::NotificationPreference;

/// Repository for notification CRUD operations.
#[derive(Debug, Clone)]
pub struct NotificationRepository {
    pool: PgPool,
}

impl NotificationRepository {
    /// Create a new notification repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List notifications for a user.
    pub async fn find_by_user(
        &self,
        user_id: Uuid,
        page: &PageRequest,
    ) -> AppResult<PageResponse<Notification>> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND (is_dismissed IS NULL OR is_dismissed = FALSE)"
        )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count notifications", e))?;

        let notifs = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE user_id = $1 AND (is_dismissed IS NULL OR is_dismissed = FALSE) \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
            .bind(user_id)
            .bind(page.limit() as i64)
            .bind(page.offset() as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list notifications", e))?;

        Ok(PageResponse::new(
            notifs,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Count unread notifications for a user.
    pub async fn count_unread(&self, user_id: Uuid) -> AppResult<u64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND (is_read IS NULL OR is_read = FALSE)"
        )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count unread", e))?;
        Ok(count as u64)
    }

    /// Create a notification.
    pub async fn create(
        &self,
        user_id: Uuid,
        category: &str,
        event_type: &str,
        title: &str,
        message: &str,
        payload: Option<&serde_json::Value>,
        priority: &str,
        actor_id: Option<Uuid>,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
    ) -> AppResult<Notification> {
        sqlx::query_as::<_, Notification>(
            "INSERT INTO notifications (user_id, category, event_type, title, message, payload, priority, actor_id, resource_type, resource_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *"
        )
            .bind(user_id)
            .bind(category)
            .bind(event_type)
            .bind(title)
            .bind(message)
            .bind(payload)
            .bind(priority)
            .bind(actor_id)
            .bind(resource_type)
            .bind(resource_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create notification", e))
    }

    /// Mark a notification as read.
    pub async fn mark_read(&self, notification_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE notifications SET is_read = TRUE, read_at = NOW() WHERE id = $1")
            .bind(notification_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to mark read", e))?;
        Ok(())
    }

    /// Mark all notifications as read for a user.
    pub async fn mark_all_read(&self, user_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE notifications SET is_read = TRUE, read_at = NOW() \
             WHERE user_id = $1 AND (is_read IS NULL OR is_read = FALSE)",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to mark all read", e))?;
        Ok(result.rows_affected())
    }

    /// Dismiss a notification.
    pub async fn dismiss(&self, notification_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE notifications SET is_dismissed = TRUE WHERE id = $1")
            .bind(notification_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to dismiss notification", e)
            })?;
        Ok(())
    }

    /// Get notification preferences for a user.
    pub async fn get_preferences(
        &self,
        user_id: Uuid,
    ) -> AppResult<Option<NotificationPreference>> {
        sqlx::query_as::<_, NotificationPreference>(
            "SELECT * FROM notification_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to get preferences", e))
    }

    /// Upsert notification preferences.
    pub async fn upsert_preferences(
        &self,
        user_id: Uuid,
        preferences: &serde_json::Value,
    ) -> AppResult<NotificationPreference> {
        sqlx::query_as::<_, NotificationPreference>(
            "INSERT INTO notification_preferences (user_id, preferences, updated_at) \
             VALUES ($1, $2, NOW()) \
             ON CONFLICT (user_id) DO UPDATE SET preferences = $2, updated_at = NOW() \
             RETURNING *",
        )
        .bind(user_id)
        .bind(preferences)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to upsert preferences", e))
    }

    /// Clean up old notifications.
    pub async fn cleanup_old(&self, before: chrono::DateTime<chrono::Utc>) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM notifications WHERE created_at < $1")
            .bind(before)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to cleanup notifications", e)
            })?;
        Ok(result.rows_affected())
    }
}
