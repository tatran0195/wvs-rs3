//! Maps domain events to notification messages and dispatches them.

use std::sync::Arc;

use tracing::debug;
use uuid::Uuid;

use crate::message::types::OutboundMessage;
use crate::notification::dispatcher::NotificationDispatcher;
use crate::notification::formatter::NotificationFormatter;

/// Bridges domain events to the notification system.
#[derive(Debug)]
pub struct EventBridge {
    /// Notification dispatcher.
    dispatcher: Arc<NotificationDispatcher>,
}

impl EventBridge {
    /// Creates a new event bridge.
    pub fn new(dispatcher: Arc<NotificationDispatcher>) -> Self {
        Self { dispatcher }
    }

    /// Handles a file uploaded event.
    pub async fn on_file_uploaded(
        &self,
        file_id: Uuid,
        filename: &str,
        folder_name: &str,
        uploader: &str,
        subscriber_ids: &[Uuid],
    ) {
        let message =
            NotificationFormatter::file_uploaded(filename, folder_name, uploader, file_id);

        self.dispatcher
            .dispatch_to_users(subscriber_ids, "file.uploaded", Some(file_id), message)
            .await;

        // Also broadcast to folder channel
        let folder_msg =
            NotificationFormatter::file_uploaded(filename, folder_name, uploader, file_id);
        // Channel name would be constructed from folder ID — simplified here
        debug!("File uploaded event bridged to notifications");
    }

    /// Handles a file deleted event.
    pub async fn on_file_deleted(
        &self,
        file_id: Uuid,
        filename: &str,
        actor: &str,
        subscriber_ids: &[Uuid],
    ) {
        let message = NotificationFormatter::file_deleted(filename, actor, file_id);

        self.dispatcher
            .dispatch_to_users(subscriber_ids, "file.deleted", Some(file_id), message)
            .await;
    }

    /// Handles a share created event.
    pub async fn on_share_created(
        &self,
        share_id: Uuid,
        resource_name: &str,
        sharer: &str,
        target_user_id: Uuid,
    ) {
        let message = NotificationFormatter::share_created(resource_name, sharer, share_id);

        self.dispatcher
            .dispatch_to_user(target_user_id, "share.created", Some(share_id), message)
            .await;
    }

    /// Handles a session terminated event.
    pub async fn on_session_terminated(
        &self,
        session_id: Uuid,
        reason: &str,
        terminated_by: Option<Uuid>,
    ) {
        self.dispatcher
            .send_session_termination(session_id, reason, terminated_by)
            .await;
    }

    /// Handles an upload progress event.
    pub async fn on_upload_progress(
        &self,
        user_id: Uuid,
        upload_id: Uuid,
        percent: u8,
        status: &str,
    ) {
        self.dispatcher
            .send_progress(user_id, upload_id, percent, status)
            .await;
    }

    /// Returns a reference to the dispatcher.
    pub fn dispatcher(&self) -> &Arc<NotificationDispatcher> {
        &self.dispatcher
    }
}
