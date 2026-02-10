//! Builder helpers for constructing outbound notification messages.

use chrono::Utc;
use uuid::Uuid;

use super::types::OutboundMessage;

/// Build a notification outbound message
pub fn build_notification(
    id: Uuid,
    category: &str,
    event_type: &str,
    title: &str,
    message: &str,
    priority: &str,
    actor_id: Option<Uuid>,
    actor_name: Option<String>,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    payload: Option<serde_json::Value>,
) -> OutboundMessage {
    OutboundMessage::Notification {
        id,
        category: category.to_string(),
        event_type: event_type.to_string(),
        title: title.to_string(),
        message: message.to_string(),
        payload,
        priority: priority.to_string(),
        actor_id,
        actor_name,
        resource_type,
        resource_id,
        timestamp: Utc::now(),
    }
}

/// Build a file created event
pub fn build_file_created(
    file_id: Uuid,
    file_name: &str,
    folder_id: Uuid,
    actor_id: Uuid,
    actor_name: &str,
    size_bytes: i64,
    mime_type: Option<String>,
) -> OutboundMessage {
    OutboundMessage::FileCreated {
        file_id,
        file_name: file_name.to_string(),
        folder_id,
        actor_id,
        actor_name: actor_name.to_string(),
        size_bytes,
        mime_type,
        timestamp: Utc::now(),
    }
}

/// Build an upload progress event
pub fn build_upload_progress(
    upload_id: Uuid,
    file_name: &str,
    chunk_number: i32,
    total_chunks: i32,
    bytes_uploaded: i64,
    total_bytes: i64,
) -> OutboundMessage {
    let percent = if total_bytes > 0 {
        (bytes_uploaded as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    OutboundMessage::UploadProgress {
        upload_id,
        file_name: file_name.to_string(),
        chunk_number,
        total_chunks,
        bytes_uploaded,
        total_bytes,
        percent,
    }
}

/// Build an admin broadcast event
pub fn build_admin_broadcast(
    broadcast_id: Uuid,
    title: &str,
    message: &str,
    severity: &str,
    persistent: bool,
) -> OutboundMessage {
    OutboundMessage::AdminBroadcast {
        broadcast_id,
        title: title.to_string(),
        message: message.to_string(),
        severity: severity.to_string(),
        persistent,
        action_type: None,
        action_payload: None,
        timestamp: Utc::now(),
    }
}

/// Build an error message
pub fn build_error(code: &str, message: &str, request_id: Option<String>) -> OutboundMessage {
    OutboundMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
        request_id,
    }
}

/// Build session terminated event
pub fn build_session_terminated(session_id: Uuid, reason: &str) -> OutboundMessage {
    OutboundMessage::SessionTerminated {
        session_id: filehub_core::types::id::SessionId::from(session_id),
        reason: reason.to_string(),
        terminated_at: Utc::now(),
    }
}
