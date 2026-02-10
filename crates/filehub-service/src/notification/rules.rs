//! Notification subscriber resolution rules — determines who should receive which notifications.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::permission::AclRepository;
use filehub_entity::permission::ResourceType;

/// Resolves which users should receive notifications for a given event.
#[derive(Debug, Clone)]
pub struct NotificationRules {
    /// ACL repository for finding users with access.
    acl_repo: Arc<AclRepository>,
}

impl NotificationRules {
    /// Creates a new notification rules engine.
    pub fn new(acl_repo: Arc<AclRepository>) -> Self {
        Self { acl_repo }
    }

    /// Gets user IDs that should be notified about a file event.
    pub async fn file_event_subscribers(
        &self,
        file_id: Uuid,
        folder_id: Uuid,
        exclude_actor: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let mut subscribers = Vec::new();

        // Get users with ACL on the file
        let file_entries = self
            .acl_repo
            .find_for_resource(ResourceType::File, file_id)
            .await
            .map_err(|e| AppError::internal(format!("ACL lookup failed: {e}")))?;

        for entry in &file_entries {
            if let Some(uid) = entry.user_id {
                if uid != exclude_actor {
                    subscribers.push(uid);
                }
            }
        }

        // Get users with ACL on the parent folder
        let folder_entries = self
            .acl_repo
            .find_for_resource(ResourceType::Folder, folder_id)
            .await
            .map_err(|e| AppError::internal(format!("ACL lookup failed: {e}")))?;

        for entry in &folder_entries {
            if let Some(uid) = entry.user_id {
                if uid != exclude_actor && !subscribers.contains(&uid) {
                    subscribers.push(uid);
                }
            }
        }

        Ok(subscribers)
    }

    /// Gets user IDs that should be notified about a folder event.
    pub async fn folder_event_subscribers(
        &self,
        folder_id: Uuid,
        exclude_actor: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let entries = self
            .acl_repo
            .find_for_resource(ResourceType::Folder, folder_id)
            .await
            .map_err(|e| AppError::internal(format!("ACL lookup failed: {e}")))?;

        let subscribers: Vec<Uuid> = entries
            .iter()
            .filter_map(|e| e.user_id)
            .filter(|uid| *uid != exclude_actor)
            .collect();

        Ok(subscribers)
    }
}
