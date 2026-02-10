//! Channel registry — manages all channels and subscriptions.

use dashmap::DashMap;

use crate::connection::handle::ConnectionId;

use super::channel::Channel;
use super::subscription::SubscriptionTracker;

/// Registry of all active pub/sub channels.
#[derive(Debug)]
pub struct ChannelRegistry {
    /// Channel name → Channel.
    channels: DashMap<String, Channel>,
    /// Subscription tracker (reverse index).
    subscriptions: SubscriptionTracker,
    /// Default buffer size.
    _buffer_size: usize,
}

impl ChannelRegistry {
    /// Creates a new channel registry.
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: DashMap::new(),
            subscriptions: SubscriptionTracker::new(),
            _buffer_size: buffer_size,
        }
    }

    /// Subscribes a connection to a channel.
    pub fn subscribe(&self, channel_name: String, conn_id: ConnectionId) {
        self.channels
            .entry(channel_name.clone())
            .or_insert_with(|| Channel::new(channel_name.clone()))
            .subscribe(conn_id);

        self.subscriptions.add(conn_id, channel_name);
    }

    /// Unsubscribes a connection from a channel.
    pub fn unsubscribe(&self, channel_name: String, conn_id: ConnectionId) {
        if let Some(mut channel) = self.channels.get_mut(&channel_name) {
            channel.unsubscribe(conn_id);
            if channel.is_empty() {
                drop(channel);
                self.channels.remove(&channel_name);
            }
        }
        self.subscriptions.remove(conn_id, &channel_name);
    }

    /// Unsubscribes a connection from all channels.
    pub fn unsubscribe_all(&self, conn_id: ConnectionId) {
        let channels = self.subscriptions.remove_all(conn_id);
        for channel_name in &channels {
            if let Some(mut channel) = self.channels.get_mut(channel_name) {
                channel.unsubscribe(conn_id);
                if channel.is_empty() {
                    drop(channel);
                    self.channels.remove(channel_name);
                }
            }
        }
    }

    /// Returns all subscriber connection IDs for a channel.
    pub fn get_subscribers(&self, channel_name: &str) -> Vec<ConnectionId> {
        self.channels
            .get(channel_name)
            .map(|ch| ch.get_subscribers())
            .unwrap_or_default()
    }

    /// Returns the subscription count for a connection.
    pub fn subscription_count(&self, conn_id: ConnectionId) -> usize {
        self.subscriptions.count(conn_id)
    }

    /// Returns subscriber count for a channel.
    pub fn channel_subscriber_count(&self, channel_name: &str) -> usize {
        self.channels
            .get(channel_name)
            .map(|ch| ch.subscriber_count())
            .unwrap_or(0)
    }

    /// Returns total number of active channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}
