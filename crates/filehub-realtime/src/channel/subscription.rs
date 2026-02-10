//! Subscription tracking — which connections are subscribed to which channels.

use std::collections::HashSet;

use dashmap::DashMap;

use crate::connection::handle::ConnectionId;

/// Tracks connection-to-channel subscription mappings (reverse index).
#[derive(Debug)]
pub struct SubscriptionTracker {
    /// Connection ID → set of channel names.
    conn_to_channels: DashMap<ConnectionId, HashSet<String>>,
}

impl SubscriptionTracker {
    /// Creates a new subscription tracker.
    pub fn new() -> Self {
        Self {
            conn_to_channels: DashMap::new(),
        }
    }

    /// Records a subscription.
    pub fn add(&self, conn_id: ConnectionId, channel: String) {
        self.conn_to_channels
            .entry(conn_id)
            .or_default()
            .insert(channel);
    }

    /// Removes a subscription.
    pub fn remove(&self, conn_id: ConnectionId, channel: &str) {
        if let Some(mut channels) = self.conn_to_channels.get_mut(&conn_id) {
            channels.remove(channel);
        }
    }

    /// Gets all channels a connection is subscribed to.
    pub fn get_channels(&self, conn_id: ConnectionId) -> HashSet<String> {
        self.conn_to_channels
            .get(&conn_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Returns the number of subscriptions for a connection.
    pub fn count(&self, conn_id: ConnectionId) -> usize {
        self.conn_to_channels
            .get(&conn_id)
            .map(|entry| entry.value().len())
            .unwrap_or(0)
    }

    /// Removes all subscriptions for a connection.
    pub fn remove_all(&self, conn_id: ConnectionId) -> HashSet<String> {
        self.conn_to_channels
            .remove(&conn_id)
            .map(|(_, channels)| channels)
            .unwrap_or_default()
    }
}

impl Default for SubscriptionTracker {
    fn default() -> Self {
        Self::new()
    }
}
