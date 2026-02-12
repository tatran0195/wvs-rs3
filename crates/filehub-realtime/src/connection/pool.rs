//! Connection pool — maps users to their active connections.

use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use filehub_core::types::id::UserId;

use super::handle::{ConnectionHandle, ConnectionId};

/// Pool of all active WebSocket connections, indexed by user.
#[derive(Debug)]
pub struct ConnectionPool {
    /// User ID → list of connection handles
    by_user: DashMap<Uuid, Vec<Arc<ConnectionHandle>>>,
    /// Connection ID → connection handle (for direct lookup)
    by_id: DashMap<ConnectionId, Arc<ConnectionHandle>>,
}

impl ConnectionPool {
    /// Create a new empty pool
    pub fn new() -> Self {
        Self {
            by_user: DashMap::new(),
            by_id: DashMap::new(),
        }
    }

    /// Add a connection to the pool
    pub fn add(&self, handle: Arc<ConnectionHandle>) {
        let user_id = handle.user_id;
        self.by_id.insert(handle.id, Arc::clone(&handle));
        self.by_user
            .entry(user_id.into_uuid())
            .or_insert_with(Vec::new)
            .push(handle);
    }

    /// Remove a connection by ID
    pub fn remove(&self, connection_id: ConnectionId) -> Option<Arc<ConnectionHandle>> {
        let handle = self.by_id.remove(&connection_id).map(|(_, h)| h)?;
        let user_id = handle.user_id;

        if let Some(mut conns) = self.by_user.get_mut(&user_id.into_uuid()) {
            conns.retain(|c| c.id != connection_id);
            if conns.is_empty() {
                drop(conns);
                self.by_user.remove(&user_id.into_uuid());
            }
        }

        Some(handle)
    }

    /// Get all connections for a user
    pub fn get_user_connections(&self, user_id: UserId) -> Vec<Arc<ConnectionHandle>> {
        self.by_user
            .get(&user_id.into_uuid())
            .map(|conns| conns.clone())
            .unwrap_or_default()
    }

    /// Get a connection by ID
    pub fn get(&self, connection_id: ConnectionId) -> Option<Arc<ConnectionHandle>> {
        self.by_id.get(&connection_id).map(|h| Arc::clone(&h))
    }

    /// Count connections for a user
    pub fn user_connection_count(&self, user_id: UserId) -> usize {
        self.by_user
            .get(&user_id.into_uuid())
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    /// Get all connection handles (for broadcast)
    pub fn all_connections(&self) -> Vec<Arc<ConnectionHandle>> {
        self.by_id.iter().map(|r| Arc::clone(r.value())).collect()
    }

    /// Get all connected user IDs
    pub fn connected_user_ids(&self) -> Vec<Uuid> {
        self.by_user.iter().map(|r| *r.key()).collect()
    }

    /// Total connection count
    pub fn total_count(&self) -> usize {
        self.by_id.len()
    }

    /// Total unique user count
    pub fn unique_user_count(&self) -> usize {
        self.by_user.len()
    }

    /// Remove dead connections
    pub fn prune_dead(&self) -> usize {
        let dead: Vec<ConnectionId> = self
            .by_id
            .iter()
            .filter(|r| !r.value().is_alive())
            .map(|r| *r.key())
            .collect();

        let count = dead.len();
        for id in dead {
            self.remove(id);
        }
        count
    }

    /// Get connections subscribed to a channel
    pub async fn subscribed_to(&self, channel: &str) -> Vec<Arc<ConnectionHandle>> {
        let mut result = Vec::new();
        for entry in self.by_id.iter() {
            if entry.value().is_subscribed(channel).await {
                result.push(Arc::clone(entry.value()));
            }
        }
        result
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
