//! Top-level real-time engine that ties together all subsystems.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::info;

use filehub_core::config::RealtimeConfig;
use filehub_core::error::AppError;

use crate::bridge::event_bridge::EventBridge;
use crate::channel::registry::ChannelRegistry;
use crate::connection::manager::ConnectionManager;
use crate::metrics::RealtimeMetrics;
use crate::notification::dispatcher::NotificationDispatcher;
use crate::presence::tracker::PresenceTracker;
use crate::session_control::monitor::SessionMonitor;

/// Central real-time engine that coordinates all WebSocket subsystems.
#[derive(Clone)]
pub struct RealtimeEngine {
    /// Connection manager.
    pub connections: Arc<ConnectionManager>,
    /// Channel registry.
    pub channels: Arc<ChannelRegistry>,
    /// Notification dispatcher.
    pub notifications: Arc<NotificationDispatcher>,
    /// Presence tracker.
    pub presence: Arc<PresenceTracker>,
    /// Session monitor (admin).
    pub session_monitor: Arc<SessionMonitor>,
    /// Event bridge (domain events → notifications).
    pub event_bridge: Arc<EventBridge>,
    /// Metrics collector.
    pub metrics: Arc<RealtimeMetrics>,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl std::fmt::Debug for RealtimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealtimeEngine").finish()
    }
}

impl RealtimeEngine {
    /// Creates a new real-time engine with all subsystems.
    pub fn new(config: RealtimeConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        let metrics = Arc::new(RealtimeMetrics::new());
        let channels = Arc::new(ChannelRegistry::new(config.channel_buffer_size));
        let presence = Arc::new(PresenceTracker::new());
        let connections = Arc::new(ConnectionManager::new(
            config.clone(),
            channels.clone(),
            presence.clone(),
            metrics.clone(),
        ));
        let notifications = Arc::new(NotificationDispatcher::new(
            connections.clone(),
            config.clone(),
        ));
        let session_monitor = Arc::new(SessionMonitor::new(connections.clone(), channels.clone()));
        let event_bridge = Arc::new(EventBridge::new(notifications.clone()));

        info!("Real-time engine initialized");

        Self {
            connections,
            channels,
            notifications,
            presence,
            session_monitor,
            event_bridge,
            metrics,
            shutdown_tx,
        }
    }

    /// Returns a shutdown receiver for graceful shutdown coordination.
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Initiates a graceful shutdown of the real-time engine.
    pub async fn shutdown(&self) -> Result<(), AppError> {
        info!("Shutting down real-time engine");

        // Signal all tasks to stop
        let _ = self.shutdown_tx.send(());

        // Close all connections
        self.connections.close_all().await;

        info!("Real-time engine shut down");
        Ok(())
    }
}
