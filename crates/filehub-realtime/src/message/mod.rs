//! WebSocket message types, serialization, and validation.

pub mod builder;
pub mod envelope;
pub mod serializer;
pub mod types;
pub mod validator;

pub use builder::NotificationBuilder;
pub use envelope::MessageEnvelope;
pub use types::{InboundMessage, OutboundMessage};
