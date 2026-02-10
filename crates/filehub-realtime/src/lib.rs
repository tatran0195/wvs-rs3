//! # filehub-realtime
//!
//! Real-time WebSocket engine for Suzuki FileHub. Provides:
//!
//! - WebSocket connection management with JWT authentication
//! - Pub/sub channel system with typed channels
//! - Real-time notification dispatch with deduplication
//! - User presence tracking (online/idle/away/dnd/offline)
//! - Admin session monitoring and broadcast
//! - Multi-node support via Redis pub/sub bridge

pub mod bridge;
pub mod channel;
pub mod connection;
pub mod message;
pub mod metrics;
pub mod notification;
pub mod presence;
pub mod server;
pub mod session_control;

pub use channel::registry::ChannelRegistry;
pub use connection::manager::ConnectionManager;
pub use notification::dispatcher::NotificationDispatcher;
pub use presence::tracker::PresenceTracker;
pub use server::RealtimeEngine;
pub use session_control::monitor::SessionMonitor;
