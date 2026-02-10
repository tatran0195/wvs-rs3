//! Notification persistence for offline users.

use std::sync::Arc;

use chrono::Utc;
use tracing::{error, info};
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::notification::NotificationRepository;
use filehub_entity::notification::Notification;

/// Persists notifications for users who are offline.
#[derive(Debug, Clone)]
pub struct NotificationPersistence {
    /// Notification repository.
    repo: Arc<NotificationRepository>,
    /// Maximum stored notifications per user.
    max_per_user: usize,
}

impl NotificationPersistence {
    /// Creates a new persistence handler.
    pub fn new(repo: Arc<NotificationRepository>, max_per_user: usize) -> Self {
        Self { repo, max_per_user }
    }

    /// Persists a notification for an offline user.
    pub async fn persist_for_user(
        &self,
        user_id: Uuid,
        category: &str,
        event_type: &str,
        title: &str,
        message: &str,
        payload: Option<serde_json::Value>,
        priority: &str,
        actor_id: Option<Uuid>,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let notification = Notification {
            id: Uuid::new_v4(),
            user_id,
            category: category.to_string(),
            event_type: event_type.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            payload,
            priority: priority.to_string(),
            is_read: false,
            read_at: None,
            is_dismissed: false,
            actor_id,
            resource_type: resource_type.map(String::from),
            resource_id,
            created_at: Utc::now(),
            expires_at: None,
        };

        self.repo
            .create(&notification)
            .await
            .map_err(|e| AppError::internal(format!("Failed to persist notification: {e}")))?;

        Ok(())
    }
}
