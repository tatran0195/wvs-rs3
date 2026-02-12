//! WebSocket real-time engine for FileHub.
//!
//! Provides:
//! - WebSocket connection management with JWT authentication
//! - Typed pub/sub channels with ACL-aware subscriptions
//! - Real-time notification dispatch (online → WS, offline → persist)
//! - User presence tracking (online/idle/away/dnd/offline)
//! - Admin session monitoring and control
//! - Domain event → notification bridging

pub mod channel;
pub mod connection;
pub mod message;
pub mod metrics;
pub mod notification;
pub mod presence;
pub mod server;
pub mod session_control;

pub use server::RealtimeEngine;
