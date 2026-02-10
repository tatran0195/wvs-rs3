//! User notification preference checking.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::notification::NotificationRepository;

/// Checks user notification preferences to decide delivery.
#[derive(Debug, Clone)]
pub struct PreferenceChecker {
    /// Notification repository for preference lookups.
    repo: Arc<NotificationRepository>,
}

impl PreferenceChecker {
    /// Creates a new preference checker.
    pub fn new(repo: Arc<NotificationRepository>) -> Self {
        Self { repo }
    }

    /// Checks whether a user wants to receive notifications of a given category and event type.
    pub async fn should_deliver(
        &self,
        user_id: Uuid,
        category: &str,
        event_type: &str,
    ) -> Result<bool, AppError> {
        let prefs = self
            .repo
            .get_preferences(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Preference lookup failed: {e}")))?;

        // Check if category is muted
        if let Some(muted) = prefs.preferences.get("muted_categories") {
            if let Some(muted_list) = muted.as_array() {
                for item in muted_list {
                    if item.as_str() == Some(category) {
                        return Ok(false);
                    }
                }
            }
        }

        // Check if specific event type is muted
        if let Some(muted) = prefs.preferences.get("muted_events") {
            if let Some(muted_list) = muted.as_array() {
                for item in muted_list {
                    if item.as_str() == Some(event_type) {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }
}
