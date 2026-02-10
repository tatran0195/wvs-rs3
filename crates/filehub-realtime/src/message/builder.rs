//! Builder pattern for constructing notification messages.

use chrono::Utc;
use uuid::Uuid;

use super::types::OutboundMessage;

/// Builder for notification messages.
#[derive(Debug, Clone)]
pub struct NotificationBuilder {
    category: String,
    event_type: String,
    title: String,
    message: String,
    payload: Option<serde_json::Value>,
    priority: String,
}

impl NotificationBuilder {
    /// Creates a new notification builder.
    pub fn new(category: &str, event_type: &str) -> Self {
        Self {
            category: category.to_string(),
            event_type: event_type.to_string(),
            title: String::new(),
            message: String::new(),
            payload: None,
            priority: "normal".to_string(),
        }
    }

    /// Sets the notification title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Sets the notification message body.
    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    /// Sets the notification payload.
    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Sets the priority (low, normal, high, urgent).
    pub fn priority(mut self, priority: &str) -> Self {
        self.priority = priority.to_string();
        self
    }

    /// Builds the outbound notification message.
    pub fn build(self) -> OutboundMessage {
        OutboundMessage::Notification {
            id: Uuid::new_v4(),
            category: self.category,
            event_type: self.event_type,
            title: self.title,
            message: self.message,
            payload: self.payload,
            priority: self.priority,
            timestamp: Utc::now(),
        }
    }
}
