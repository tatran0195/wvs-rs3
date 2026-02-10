//! Format domain events into notification messages.

use chrono::Utc;
use uuid::Uuid;

use crate::message::types::OutboundMessage;

/// Format a file event into a user notification
pub fn format_file_notification(
    event_type: &str,
    file_name: &str,
    actor_name: &str,
    actor_id: Uuid,
    file_id: Uuid,
) -> OutboundMessage {
    let (title, message) = match event_type {
        "file_created" => (
            "File uploaded".to_string(),
            format!("{} uploaded '{}'", actor_name, file_name),
        ),
        "file_updated" => (
            "File updated".to_string(),
            format!("{} updated '{}'", actor_name, file_name),
        ),
        "file_deleted" => (
            "File deleted".to_string(),
            format!("{} deleted '{}'", actor_name, file_name),
        ),
        "file_moved" => (
            "File moved".to_string(),
            format!("{} moved '{}'", actor_name, file_name),
        ),
        _ => (
            "File event".to_string(),
            format!(
                "{} performed '{}' on '{}'",
                actor_name, event_type, file_name
            ),
        ),
    };

    OutboundMessage::Notification {
        id: Uuid::new_v4(),
        category: "file".to_string(),
        event_type: event_type.to_string(),
        title,
        message,
        payload: None,
        priority: "normal".to_string(),
        actor_id: Some(actor_id),
        actor_name: Some(actor_name.to_string()),
        resource_type: Some("file".to_string()),
        resource_id: Some(file_id),
        timestamp: Utc::now(),
    }
}

/// Format a share event into a notification
pub fn format_share_notification(
    event_type: &str,
    resource_name: &str,
    actor_name: &str,
    actor_id: Uuid,
    share_id: Uuid,
) -> OutboundMessage {
    let (title, message) = match event_type {
        "share_created" => (
            "New share".to_string(),
            format!("{} shared '{}' with you", actor_name, resource_name),
        ),
        "share_accessed" => (
            "Share accessed".to_string(),
            format!("Your share of '{}' was accessed", resource_name),
        ),
        _ => (
            "Share event".to_string(),
            format!("Share event on '{}'", resource_name),
        ),
    };

    OutboundMessage::Notification {
        id: Uuid::new_v4(),
        category: "share".to_string(),
        event_type: event_type.to_string(),
        title,
        message,
        payload: None,
        priority: "normal".to_string(),
        actor_id: Some(actor_id),
        actor_name: Some(actor_name.to_string()),
        resource_type: Some("share".to_string()),
        resource_id: Some(share_id),
        timestamp: Utc::now(),
    }
}

/// Format a session event into an admin notification
pub fn format_session_notification(
    event_type: &str,
    username: &str,
    session_id: Uuid,
    details: &str,
) -> OutboundMessage {
    let (title, message) = match event_type {
        "session_created" => (
            "New session".to_string(),
            format!("User '{}' logged in", username),
        ),
        "session_terminated" => (
            "Session terminated".to_string(),
            format!("Session for '{}' was terminated: {}", username, details),
        ),
        "session_expired" => (
            "Session expired".to_string(),
            format!("Session for '{}' expired", username),
        ),
        _ => (
            "Session event".to_string(),
            format!("Session event for '{}': {}", username, details),
        ),
    };

    OutboundMessage::Notification {
        id: Uuid::new_v4(),
        category: "session".to_string(),
        event_type: event_type.to_string(),
        title,
        message,
        payload: None,
        priority: "high".to_string(),
        actor_id: None,
        actor_name: Some(username.to_string()),
        resource_type: Some("session".to_string()),
        resource_id: Some(session_id),
        timestamp: Utc::now(),
    }
}
