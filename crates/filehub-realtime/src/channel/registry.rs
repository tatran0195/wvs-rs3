//! Channel registry — creates and manages all channels.

use std::sync::Arc;

use dashmap::DashMap;
use tracing;

use crate::connection::handle::ConnectionId;

use super::channel::Channel;
use super::types::ChannelType;

/// Registry of all active channels.
#[derive(Debug)]
pub struct ChannelRegistry {
    /// Active channels by name
    channels: DashMap<String, Arc<Channel>>,
    /// Default buffer size for new channels
    _buffer_size: usize,
}

impl ChannelRegistry {
    /// Create a new channel registry
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: DashMap::new(),
            _buffer_size: buffer_size,
        }
    }

    /// Get or create a channel
    pub fn get_or_create(&self, channel_type: ChannelType) -> Arc<Channel> {
        let name = channel_type.to_channel_name();
        self.channels
            .entry(name.clone())
            .or_insert_with(|| {
                tracing::debug!("Channel created: {}", name);
                Arc::new(Channel::new(channel_type))
            })
            .clone()
    }

    /// Get a channel by name (does not create)
    pub fn get(&self, name: &str) -> Option<Arc<Channel>> {
        self.channels.get(name).map(|r| Arc::clone(&r))
    }

    /// Subscribe a connection to a channel
    pub fn subscribe(&self, channel_name: &str, connection_id: ConnectionId) -> bool {
        let channel_type = match ChannelType::parse(channel_name) {
            Some(ct) => ct,
            None => {
                tracing::warn!("Invalid channel name: {}", channel_name);
                return false;
            }
        };

        let channel = self.get_or_create(channel_type);
        channel.subscribe(connection_id)
    }

    /// Unsubscribe a connection from a channel
    pub fn unsubscribe(&self, channel_name: &str, connection_id: ConnectionId) -> bool {
        if let Some(channel) = self.get(channel_name) {
            let removed = channel.unsubscribe(connection_id);
            // Clean up empty channels (except well-known ones)
            if !channel.has_subscribers() {
                let ct = &channel.channel_type;
                if !ct.is_public() && !ct.requires_admin() {
                    self.channels.remove(channel_name);
                    tracing::debug!("Channel removed (no subscribers): {}", channel_name);
                }
            }
            removed
        } else {
            false
        }
    }

    /// Remove a connection from ALL channels
    pub fn unsubscribe_all(&self, connection_id: ConnectionId) {
        for entry in self.channels.iter() {
            entry.value().unsubscribe(connection_id);
        }
    }

    /// Get subscriber IDs for a channel
    pub fn subscribers(&self, channel_name: &str) -> Vec<ConnectionId> {
        self.get(channel_name)
            .map(|ch| ch.subscriber_ids())
            .unwrap_or_default()
    }

    /// Get all channel names
    pub fn channel_names(&self) -> Vec<String> {
        self.channels.iter().map(|r| r.key().clone()).collect()
    }

    /// Get total channel count
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}
