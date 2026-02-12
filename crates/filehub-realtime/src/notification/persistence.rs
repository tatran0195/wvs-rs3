//! Notification persistence for offline users.

use filehub_core::error::AppError;
use filehub_core::types::id::UserId;

use crate::message::types::OutboundMessage;

/// Store a notification for an offline user.
///
/// The notification will be delivered when the user comes back online
/// or fetched via the REST API.
pub async fn persist_for_offline(
    notification_service: &filehub_service::notification::service::NotificationService,
    user_id: UserId,
    msg: &OutboundMessage,
) -> Result<(), AppError> {
    if let OutboundMessage::Notification {
        id,
        category,
        event_type,
        title,
        message,
        payload,
        priority,
        actor_id,
        resource_type,
        resource_id,
        timestamp,
        ..
    } = msg
    {
        let notification = filehub_entity::notification::model::Notification {
            id: *id,
            user_id: user_id.into_uuid(),
            category: category.clone(),
            event_type: event_type.clone(),
            title: title.clone(),
            message: message.clone(),
            payload: payload.clone(),
            priority: Some(priority.clone()),
            is_read: Some(false),
            read_at: None,
            is_dismissed: Some(false),
            actor_id: *actor_id,
            resource_type: resource_type.clone(),
            resource_id: *resource_id,
            created_at: *timestamp,
            expires_at: None,
        };

        notification_service
            .create_notification(notification)
            .await?;
    }

    Ok(())
}
