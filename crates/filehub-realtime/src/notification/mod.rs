//! Notification dispatch system.

pub mod dedup;
pub mod dispatcher;
pub mod formatter;
pub mod persistence;
pub mod preferences;
pub mod priority;

pub use dispatcher::NotificationDispatcher;
