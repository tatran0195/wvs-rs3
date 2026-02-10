//! Single channel with subscriber tracking.

use std::collections::HashSet;

use crate::connection::handle::ConnectionId;

/// A single pub/sub channel with a set of subscribers.
#[derive(Debug, Clone)]
pub struct Channel {
    /// Channel name.
    pub name: String,
    /// Set of subscribed connection IDs.
    pub subscribers: HashSet<ConnectionId>,
}

impl Channel {
    /// Creates a new empty channel.
    pub fn new(name: String) -> Self {
        Self {
            name,
            subscribers: HashSet::new(),
        }
    }

    /// Adds a subscriber.
    pub fn subscribe(&mut self, conn_id: ConnectionId) {
        self.subscribers.insert(conn_id);
    }

    /// Removes a subscriber.
    pub fn unsubscribe(&mut self, conn_id: ConnectionId) {
        self.subscribers.remove(&conn_id);
    }

    /// Returns subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Returns whether the channel has any subscribers.
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Returns all subscriber connection IDs.
    pub fn get_subscribers(&self) -> Vec<ConnectionId> {
        self.subscribers.iter().copied().collect()
    }
}
