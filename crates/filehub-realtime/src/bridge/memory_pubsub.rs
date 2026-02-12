//! In-memory pub/sub for single-node deployments.

use std::collections::HashMap;

use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::message::types::OutboundMessage;

/// In-memory pub/sub implementation.
#[derive(Debug)]
pub struct MemoryPubSub {
    /// Channel name → broadcast sender
    channels: RwLock<HashMap<String, broadcast::Sender<OutboundMessage>>>,
    /// Buffer size for channels
    buffer_size: usize,
}

impl MemoryPubSub {
    /// Create a new in-memory pub/sub
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            buffer_size,
        }
    }

    /// Publish a message to a channel
    pub async fn publish(&self, channel: &str, msg: OutboundMessage) {
        let channels = self.channels.read().await;
        if let Some(tx) = channels.get(channel) {
            let _ = tx.send(msg);
        }
    }

    /// Subscribe to a channel, returns a receiver
    pub async fn subscribe(&self, channel: &str) -> broadcast::Receiver<OutboundMessage> {
        let mut channels = self.channels.write().await;
        let tx = channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(self.buffer_size).0);
        tx.subscribe()
    }
}
