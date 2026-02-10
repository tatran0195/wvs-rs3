//! FlexNet plugin implementation — registers with the FileHub plugin system.

use std::sync::Arc;

use async_trait::async_trait;
use tracing;

use filehub_core::config::LicenseConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_plugin::hooks::definitions::HookHandler;
use filehub_plugin::hooks::registry::HookRegistry;
use filehub_plugin::registry::PluginInfo;

use crate::ffi::bindings::FlexNetBindings;
use crate::ffi::wrapper::FlexNetWrapper;
use crate::hooks::{
    AfterLoginHook, AfterSessionTerminateHook, BeforeLogoutHook, OnSessionExpiredHook,
    OnSessionIdleHook,
};
use crate::license::manager::LicenseManager;
use crate::license::pool::PoolSyncService;

/// FlexNet license plugin for FileHub
#[derive(Debug)]
pub struct FlexNetPlugin {
    /// Plugin information
    info: PluginInfo,
    /// License manager (initialized on load)
    manager: Option<Arc<LicenseManager>>,
    /// Pool sync service handle
    pool_sync_cancel: Option<tokio::sync::watch::Sender<bool>>,
}

impl FlexNetPlugin {
    /// Create a new FlexNet plugin
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "flexnet".to_string(),
                version: "1.0.0".to_string(),
                description: "FlexNet Publisher license integration".to_string(),
                author: "Suzuki FileHub".to_string(),
            },
            manager: None,
            pool_sync_cancel: None,
        }
    }

    /// Initialize the plugin with dependencies
    pub async fn initialize(
        &mut self,
        config: LicenseConfig,
        bindings: Arc<dyn FlexNetBindings>,
        checkout_repo: Arc<LicenseCheckoutRepository>,
        snapshot_repo: Arc<PoolSnapshotRepository>,
    ) -> Result<Arc<LicenseManager>, AppError> {
        let wrapper = Arc::new(FlexNetWrapper::new(bindings));
        let manager = Arc::new(LicenseManager::new(
            wrapper,
            config.clone(),
            checkout_repo,
            snapshot_repo,
        ));

        manager.initialize().await?;

        let (tx, rx) = tokio::sync::watch::channel(false);
        let sync_service =
            PoolSyncService::new(Arc::clone(&manager), config.pool.refresh_interval_seconds);

        tokio::spawn(async move {
            sync_service.run(rx).await;
        });

        self.pool_sync_cancel = Some(tx);
        self.manager = Some(Arc::clone(&manager));

        tracing::info!("FlexNet plugin initialized successfully");
        Ok(manager)
    }

    /// Register all hooks with the hook registry
    pub fn register_hooks(&self, registry: &mut HookRegistry) -> Result<(), AppError> {
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| AppError::internal("FlexNet plugin not initialized"))?;

        registry.register(
            "after_login",
            Arc::new(AfterLoginHook::new(Arc::clone(manager))),
        );

        registry.register(
            "before_logout",
            Arc::new(BeforeLogoutHook::new(Arc::clone(manager))),
        );

        registry.register(
            "after_session_terminate",
            Arc::new(AfterSessionTerminateHook::new(Arc::clone(manager))),
        );

        registry.register(
            "on_session_expired",
            Arc::new(OnSessionExpiredHook::new(Arc::clone(manager))),
        );

        registry.register(
            "on_session_idle",
            Arc::new(OnSessionIdleHook::new(Arc::clone(manager), true)),
        );

        tracing::info!(
            "FlexNet hooks registered: after_login, before_logout, after_session_terminate, on_session_expired, on_session_idle"
        );
        Ok(())
    }

    /// Get the license manager
    pub fn manager(&self) -> Option<&Arc<LicenseManager>> {
        self.manager.as_ref()
    }

    /// Get plugin info
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Shutdown the plugin
    pub async fn shutdown(&mut self) -> Result<(), AppError> {
        tracing::info!("Shutting down FlexNet plugin");

        if let Some(tx) = self.pool_sync_cancel.take() {
            let _ = tx.send(true);
        }

        if let Some(ref manager) = self.manager {
            manager.shutdown().await?;
        }

        self.manager = None;
        tracing::info!("FlexNet plugin shut down successfully");
        Ok(())
    }
}

impl Default for FlexNetPlugin {
    fn default() -> Self {
        Self::new()
    }
}
