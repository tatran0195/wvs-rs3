//! Pool status synchronization service.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time;
use tracing;

use super::manager::LicenseManager;

/// Service that periodically syncs pool status from the DLL
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

    /// Start the pool sync loop (runs until cancelled)
    pub async fn run(&self, mut cancel: watch::Receiver<bool>) {
        tracing::info!(
            "Pool sync service started, interval={}s",
            self.interval.as_secs()
        );

        let mut interval = time::interval(self.interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.manager.sync_pool_status().await {
                        Ok(status) => {
                            tracing::trace!(
                                "Pool sync: total={}, used={}, available={}",
                                status.total_seats,
                                status.checked_out,
                                status.available
                            );
                        }
                        Err(e) => {
                            tracing::error!("Pool sync failed: {}", e);
                        }
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
