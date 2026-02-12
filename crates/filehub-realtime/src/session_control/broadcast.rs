//! Admin broadcast message delivery.

use std::sync::Arc;

use uuid::Uuid;

use crate::connection::manager::ConnectionManager;
use crate::message::builder;

/// Send an admin broadcast to all connected users.
pub async fn send_broadcast(
    connections: &Arc<ConnectionManager>,
    broadcast_id: Uuid,
    title: &str,
    message: &str,
    severity: &str,
    persistent: bool,
) -> usize {
    let msg = builder::build_admin_broadcast(broadcast_id, title, message, severity, persistent);

    let total = connections.total_connections();
    connections.broadcast(msg).await;
    total
}
