//! Connection pool — tracks all active connections indexed by user ID.

use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use super::handle::{ConnectionHandle, ConnectionId};

/// Thread-safe pool of all active WebSocket connections.
#[derive(Debug)]
pub struct ConnectionPool {
    /// User ID → list of connection handles (one user can have multiple connections).
    by_user: DashMap<Uuid, Vec<Arc<ConnectionHandle>>>,
    /// Connection ID → connection handle for direct lookup.
    by_id: DashMap<ConnectionId, Arc<ConnectionHandle>>,
}

impl ConnectionPool {
    /// Creates a new empty connection pool.
    pub fn new() -> Self {
        Self {
            by_user: DashMap::new(),
            by_id: DashMap::new(),
        }
    }

    /// Adds a connection to the pool.
    pub fn add(&self, handle: Arc<ConnectionHandle>) {
        self.by_id.insert(handle.id, handle.clone());
        self.by_user.entry(handle.user_id).or_default().push(handle);
    }

    /// Removes a connection from the pool.
    pub fn remove(&self, conn_id: &ConnectionId) -> Option<Arc<ConnectionHandle>> {
        if let Some((_, handle)) = self.by_id.remove(conn_id) {
            if let Some(mut connections) = self.by_user.get_mut(&handle.user_id) {
                connections.retain(|c| c.id != *conn_id);
                if connections.is_empty() {
                    drop(connections);
                    self.by_user.remove(&handle.user_id);
                }
            }
            Some(handle)
        } else {
            None
        }
    }

    /// Gets all connections for a user.
    pub fn get_user_connections(&self, user_id: &Uuid) -> Vec<Arc<ConnectionHandle>> {
        self.by_user
            .get(user_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Gets a specific connection by ID.
    pub fn get(&self, conn_id: &ConnectionId) -> Option<Arc<ConnectionHandle>> {
        self.by_id.get(conn_id).map(|entry| entry.value().clone())
    }

    /// Gets all connections for a session.
    pub fn get_session_connections(&self, session_id: &Uuid) -> Vec<Arc<ConnectionHandle>> {
        self.by_id
            .iter()
            .filter(|entry| entry.value().session_id == *session_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Returns total number of active connections.
    pub fn connection_count(&self) -> usize {
        self.by_id.len()
    }

    /// Returns number of unique connected users.
    pub fn user_count(&self) -> usize {
        self.by_user.len()
    }

    /// Returns all connection handles.
    pub fn all_connections(&self) -> Vec<Arc<ConnectionHandle>> {
        self.by_id
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Returns all connected user IDs.
    pub fn connected_user_ids(&self) -> Vec<Uuid> {
        self.by_user.iter().map(|entry| *entry.key()).collect()
    }

    /// Removes all connections for a user.
    pub fn remove_user(&self, user_id: &Uuid) -> Vec<Arc<ConnectionHandle>> {
        if let Some((_, connections)) = self.by_user.remove(user_id) {
            for conn in &connections {
                self.by_id.remove(&conn.id);
            }
            connections
        } else {
            Vec::new()
        }
    }

    /// Removes all connections for a session.
    pub fn remove_session(&self, session_id: &Uuid) -> Vec<Arc<ConnectionHandle>> {
        let conns: Vec<Arc<ConnectionHandle>> = self
            .by_id
            .iter()
            .filter(|entry| entry.value().session_id == *session_id)
            .map(|entry| entry.value().clone())
            .collect();

        for conn in &conns {
            self.remove(&conn.id);
        }

        conns
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
