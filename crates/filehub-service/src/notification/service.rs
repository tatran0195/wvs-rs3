//! Notification CRUD and preference management.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::notification::NotificationRepository;
use filehub_entity::notification::{Notification, NotificationPreference};

use crate::context::RequestContext;

/// Manages user notifications and preferences.
#[derive(Debug, Clone)]
pub struct NotificationService {
    /// Notification repository.
    notif_repo: Arc<NotificationRepository>,
}

impl NotificationService {
    /// Creates a new notification service.
    pub fn new(notif_repo: Arc<NotificationRepository>) -> Self {
        Self { notif_repo }
    }

    /// Lists notifications for the current user.
    pub async fn list_notifications(
        &self,
        ctx: &RequestContext,
        page: PageRequest,
    ) -> Result<PageResponse<Notification>, AppError> {
        self.notif_repo
            .find_by_user(ctx.user_id, page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list notifications: {e}")))
    }

    /// Gets the unread notification count.
    pub async fn unread_count(&self, ctx: &RequestContext) -> Result<i64, AppError> {
        self.notif_repo
            .count_unread(ctx.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to count unread: {e}")))
    }

    /// Marks a notification as read.
    pub async fn mark_read(
        &self,
        ctx: &RequestContext,
        notification_id: Uuid,
    ) -> Result<(), AppError> {
        self.notif_repo
            .mark_read(notification_id, ctx.user_id, Utc::now())
            .await
            .map_err(|e| AppError::internal(format!("Failed to mark read: {e}")))
    }

    /// Marks all notifications as read for the current user.
    pub async fn mark_all_read(&self, ctx: &RequestContext) -> Result<u64, AppError> {
        self.notif_repo
            .mark_all_read(ctx.user_id, Utc::now())
            .await
            .map_err(|e| AppError::internal(format!("Failed to mark all read: {e}")))
    }

    /// Dismisses (soft-deletes) a notification.
    pub async fn dismiss(
        &self,
        ctx: &RequestContext,
        notification_id: Uuid,
    ) -> Result<(), AppError> {
        self.notif_repo
            .dismiss(notification_id, ctx.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to dismiss notification: {e}")))
    }

    /// Creates a new notification for a user.
    pub async fn create_notification(
        &self,
        notification: Notification,
    ) -> Result<Notification, AppError> {
        self.notif_repo
            .create(&notification)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create notification: {e}")))?;

        Ok(notification)
    }

    /// Gets the user's notification preferences.
    pub async fn get_preferences(
        &self,
        ctx: &RequestContext,
    ) -> Result<NotificationPreference, AppError> {
        self.notif_repo
            .get_preferences(ctx.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get preferences: {e}")))
    }

    /// Updates the user's notification preferences.
    pub async fn update_preferences(
        &self,
        ctx: &RequestContext,
        preferences: serde_json::Value,
    ) -> Result<NotificationPreference, AppError> {
        self.notif_repo
            .upsert_preferences(ctx.user_id, preferences)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update preferences: {e}")))
    }
}
