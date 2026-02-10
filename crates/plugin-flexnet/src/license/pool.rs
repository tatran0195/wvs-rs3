//! Pool status synchronization service.

use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing;

use super::manager::LicenseManager;

/// Service that periodically syncs pool status
#[derive(Debug)]
pub struct PoolSyncService {
    /// License manager reference
    manager: Arc<LicenseManager>,
    /// Sync interval
    interval: Duration,
}

impl PoolSyncService {
    /// Create a new pool sync service
    pub fn new(manager: Arc<LicenseManager>, interval_seconds: u64) -> Self {
        Self {
            manager,
            interval: Duration::from_secs(interval_seconds),
        }
    }

    /// Start the pool sync loop (runs until the token is cancelled)
    pub async fn run(&self, cancel: tokio::sync::watch::Receiver<bool>) {
        tracing::info!(
            "Pool sync service started, interval={}s",
            self.interval.as_secs()
        );

        let mut interval = time::interval(self.interval);
        let mut cancel = cancel;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.manager.sync_pool_status().await {
                        tracing::error!("Pool sync failed: {}", e);
                    } else {
                        tracing::trace!("Pool status synced successfully");
                    }
                }
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        tracing::info!("Pool sync service shutting down");
                        break;
                    }
                }
            }
        }
    }
}
