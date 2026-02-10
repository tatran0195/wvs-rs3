//! User presence tracking — online, idle, away, dnd, offline.

pub mod activity;
pub mod status;
pub mod tracker;

pub use tracker::PresenceTracker;
