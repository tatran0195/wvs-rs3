//! In-memory pub/sub for single-node deployments.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast};

/// In-memory pub/sub channel.
#[derive(Debug)]
pub struct MemoryPubSub {
    /// Topic → broadcast sender.
    topics: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    /// Buffer size for broadcast channels.
    buffer_size: usize,
}

impl MemoryPubSub {
    /// Creates a new in-memory pub/sub system.
    pub fn new(buffer_size: usize) -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
            buffer_size,
        }
    }

    /// Publishes a message to a topic.
    pub async fn publish(&self, topic: &str, message: String) {
        let topics = self.topics.lock().await;
        if let Some(tx) = topics.get(topic) {
            let _ = tx.send(message);
        }
    }

    /// Subscribes to a topic, returning a receiver.
    pub async fn subscribe(&self, topic: &str) -> broadcast::Receiver<String> {
        let mut topics = self.topics.lock().await;
        let tx = topics
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(self.buffer_size).0);
        tx.subscribe()
    }
}
