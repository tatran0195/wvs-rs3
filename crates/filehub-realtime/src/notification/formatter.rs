//! Notification message formatting.

use uuid::Uuid;

use crate::message::builder::NotificationBuilder;
use crate::message::types::OutboundMessage;

/// Formats notification messages for common events.
pub struct NotificationFormatter;

impl NotificationFormatter {
    /// Formats a file uploaded notification.
    pub fn file_uploaded(
        filename: &str,
        folder_name: &str,
        uploader: &str,
        file_id: Uuid,
    ) -> OutboundMessage {
        NotificationBuilder::new("file", "file.uploaded")
            .title("New File Uploaded")
            .message(&format!(
                "{uploader} uploaded '{filename}' to {folder_name}"
            ))
            .payload(serde_json::json!({
                "file_id": file_id,
                "filename": filename,
                "folder": folder_name,
                "uploader": uploader,
            }))
            .build()
    }

    /// Formats a file deleted notification.
    pub fn file_deleted(filename: &str, actor: &str, file_id: Uuid) -> OutboundMessage {
        NotificationBuilder::new("file", "file.deleted")
            .title("File Deleted")
            .message(&format!("{actor} deleted '{filename}'"))
            .payload(serde_json::json!({
                "file_id": file_id,
                "filename": filename,
            }))
            .build()
    }

    /// Formats a share created notification.
    pub fn share_created(resource_name: &str, sharer: &str, share_id: Uuid) -> OutboundMessage {
        NotificationBuilder::new("share", "share.created")
            .title("New Share")
            .message(&format!("{sharer} shared '{resource_name}' with you"))
            .payload(serde_json::json!({
                "share_id": share_id,
                "resource": resource_name,
            }))
            .build()
    }

    /// Formats a session terminated notification.
    pub fn session_terminated(
        session_id: Uuid,
        reason: &str,
        terminated_by: Option<Uuid>,
    ) -> OutboundMessage {
        OutboundMessage::SessionTerminated {
            session_id,
            reason: reason.to_string(),
            terminated_by,
            grace_seconds: 5,
        }
    }

    /// Formats an admin broadcast.
    pub fn admin_broadcast(
        id: Uuid,
        title: &str,
        message: &str,
        severity: &str,
        persistent: bool,
    ) -> OutboundMessage {
        OutboundMessage::AdminBroadcast {
            id,
            title: title.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            persistent,
            action: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Formats an upload progress notification.
    pub fn upload_progress(upload_id: Uuid, percent: u8, status: &str) -> OutboundMessage {
        OutboundMessage::Progress {
            resource_id: upload_id,
            percent,
            status: status.to_string(),
            details: None,
        }
    }
}
