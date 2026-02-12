//! Individual channel with subscriber tracking.

use dashmap::DashSet;

use crate::connection::handle::ConnectionId;

use super::types::ChannelType;

/// A pub/sub channel that tracks subscribers.
#[derive(Debug)]
pub struct Channel {
    /// Channel type
    pub channel_type: ChannelType,
    /// Channel name (canonical string)
    pub name: String,
    /// Set of subscribed connection IDs
    subscribers: DashSet<ConnectionId>,
}

impl Channel {
    /// Create a new channel
    pub fn new(channel_type: ChannelType) -> Self {
        let name = channel_type.to_channel_name();
        Self {
            channel_type,
            name,
            subscribers: DashSet::new(),
        }
    }

    /// Add a subscriber
    pub fn subscribe(&self, connection_id: ConnectionId) -> bool {
        self.subscribers.insert(connection_id)
    }

    /// Remove a subscriber
    pub fn unsubscribe(&self, connection_id: ConnectionId) -> bool {
        self.subscribers.remove(&connection_id).is_some()
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Get all subscriber connection IDs
    pub fn subscriber_ids(&self) -> Vec<ConnectionId> {
        self.subscribers.iter().map(|r| *r).collect()
    }

    /// Check if a connection is subscribed
    pub fn is_subscribed(&self, connection_id: ConnectionId) -> bool {
        self.subscribers.contains(&connection_id)
    }

    /// Check if the channel has any subscribers
    pub fn has_subscribers(&self) -> bool {
        !self.subscribers.is_empty()
    }
}
