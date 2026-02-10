//! Bridges between domain events and the real-time notification system.

pub mod event_bridge;
pub mod memory_pubsub;
pub mod redis_pubsub;

pub use event_bridge::EventBridge;
