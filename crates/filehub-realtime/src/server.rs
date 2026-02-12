//! WebSocket server setup and engine initialization.

use std::sync::Arc;

use tracing;

use filehub_auth::jwt::decoder::JwtDecoder;
use filehub_core::config::RealtimeConfig;
use filehub_database::repositories::session::SessionRepository;
use filehub_service::notification::service::NotificationService;

use crate::channel::registry::ChannelRegistry;
use crate::connection::manager::ConnectionManager;
use crate::metrics::EngineMetrics;
use crate::notification::dispatcher::NotificationDispatcher;
use crate::presence::tracker::PresenceTracker;
use crate::session_control::monitor::SessionMonitor;

/// Core realtime engine holding all subsystems.
#[derive(Debug)]
pub struct RealtimeEngine {
    /// Configuration
    pub config: RealtimeConfig,
    /// Connection manager
    pub connections: Arc<ConnectionManager>,
    /// Channel registry
    pub channels: Arc<ChannelRegistry>,
    /// Notification dispatcher
    pub notifications: Arc<NotificationDispatcher>,
    /// Presence tracker
    pub presence: Arc<PresenceTracker>,
    /// Session monitor (admin)
    pub session_monitor: Arc<SessionMonitor>,
    /// Engine metrics
    pub metrics: Arc<EngineMetrics>,
    /// JWT decoder for WS auth
    pub jwt_decoder: Arc<JwtDecoder>,
    /// Session repository
    pub session_repo: Arc<SessionRepository>,
}

impl RealtimeEngine {
    /// Create and initialize the realtime engine.
    pub async fn new(
        config: &RealtimeConfig,
        jwt_decoder: Arc<JwtDecoder>,
        session_repo: Arc<SessionRepository>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        let metrics = Arc::new(EngineMetrics::new());
        let channels = Arc::new(ChannelRegistry::new(config.channel_buffer_size));
        let connections = Arc::new(ConnectionManager::new(
            config.max_connections_per_user,
            config.max_subscriptions_per_connection,
        ));
        let presence = Arc::new(PresenceTracker::new());
        let notifications = Arc::new(NotificationDispatcher::new(
            Arc::clone(&connections),
            notification_service,
            config.notifications.clone(),
        ));
        let session_monitor = Arc::new(SessionMonitor::new(Arc::clone(&connections)));

        tracing::info!(
            "Realtime engine created: max_conn_per_user={}, channel_buf={}, max_subs={}",
            config.max_connections_per_user,
            config.channel_buffer_size,
            config.max_subscriptions_per_connection,
        );

        Self {
            config: config.clone(),
            connections,
            channels,
            notifications,
            presence,
            session_monitor,
            metrics,
            jwt_decoder,
            session_repo,
        }
    }
}
