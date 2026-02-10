//! Domain events emitted by FileHub operations.
//!
//! Events are dispatched through the event bus and consumed by
//! the real-time engine, notification system, audit logger,
//! and plugin hook framework.

pub mod file;
pub mod session;
pub mod share;
pub mod system;
pub mod user;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use file::FileEvent;
pub use session::SessionEvent;
pub use share::ShareEvent;
pub use system::SystemEvent;
pub use user::UserEvent;

/// Wrapper for all domain events with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    /// Unique event ID.
    pub id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// The user who caused the event (if applicable).
    pub actor_id: Option<Uuid>,
    /// The event payload.
    pub payload: EventPayload,
}

/// Union of all domain event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "domain", content = "event")]
pub enum EventPayload {
    /// A file-related event.
    File(FileEvent),
    /// A user-related event.
    User(UserEvent),
    /// A share-related event.
    Share(ShareEvent),
    /// A session-related event.
    Session(SessionEvent),
    /// A system-level event.
    System(SystemEvent),
}

impl DomainEvent {
    /// Create a new domain event.
    pub fn new(actor_id: Option<Uuid>, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id,
            payload,
        }
    }
}
