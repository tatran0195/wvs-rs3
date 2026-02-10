//! WebSocket message types and serialization.

pub mod builder;
pub mod envelope;
pub mod serializer;
pub mod types;
pub mod validator;

pub use envelope::MessageEnvelope;
pub use types::{InboundMessage, OutboundMessage};
