//! FlexNet plugin — registers with the FileHub plugin system.

use std::path::PathBuf;
use std::sync::Arc;

use tracing;

use filehub_core::config::LicenseConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_plugin::hooks::registry::HookRegistry;
use filehub_plugin::registry::PluginInfo;

use crate::ffi::wrapper::LicenseManagerWrapper;
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
    /// License manager (set after initialization)
    manager: Option<Arc<LicenseManager>>,
    /// Pool sync cancellation sender
    pool_sync_cancel: Option<tokio::sync::watch::Sender<bool>>,
}

impl FlexNetPlugin {
    /// Create a new FlexNet plugin instance
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "flexnet".to_string(),
                version: "1.0.0".to_string(),
                description: "FlexNet Publisher license integration via license_proxy.dll"
                    .to_string(),
                author: "Suzuki FileHub".to_string(),
            },
            manager: None,
            pool_sync_cancel: None,
        }
    }

    /// Initialize the plugin.
    ///
    /// Loads the DLL (or mock), creates the license manager, starts pool sync,
    /// and returns the license manager for use by other components.
    pub async fn initialize(
        &mut self,
        config: LicenseConfig,
        dll_path: Option<PathBuf>,
        checkout_repo: Arc<LicenseCheckoutRepository>,
        snapshot_repo: Arc<PoolSnapshotRepository>,
    ) -> Result<Arc<LicenseManager>, AppError> {
        tracing::info!("Initializing FlexNet plugin");

        // Create the FFI wrapper (real or mock)
        let wrapper = LicenseManagerWrapper::create(dll_path)
            .map_err(|e| AppError::internal(format!("Failed to create license wrapper: {}", e)))?;

        let wrapper = Arc::new(wrapper);

        if wrapper.is_mock() {
            tracing::warn!("FlexNet plugin using MOCK implementation");
            // Configure mock with default seats
            #[cfg(feature = "mock")]
            wrapper.as_mock().set_total_seats(&config.feature_name, 10);
        }

        // Create the license manager
        let manager = Arc::new(LicenseManager::new(
            Arc::clone(&wrapper),
            config.clone(),
            checkout_repo,
            snapshot_repo,
        ));

        // Initialize
        manager.initialize().await?;

        // Start pool sync service
        let (tx, rx) = tokio::sync::watch::channel(false);
        let sync_service =
            PoolSyncService::new(Arc::clone(&manager), config.pool.refresh_interval_seconds);

        tokio::spawn(async move {
            sync_service.run(rx).await;
        });

        self.pool_sync_cancel = Some(tx);
        self.manager = Some(Arc::clone(&manager));

        let server_info = manager.server_info();
        let is_star = manager.is_star_license();
        let is_mock = manager.is_mock();

        tracing::info!(
            "FlexNet plugin initialized: server='{}', star={}, mock={}",
            server_info,
            is_star,
            is_mock
        );

        Ok(manager)
    }

    /// Register all hooks with the hook registry.
    ///
    /// Hooks:
    /// - `after_login` → checkout license
    /// - `before_logout` → checkin license
    /// - `after_session_terminate` → checkin license
    /// - `on_session_expired` → checkin license
    /// - `on_session_idle` → maybe release under pressure
    pub fn register_hooks(&self, registry: &mut HookRegistry) -> Result<(), AppError> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            AppError::internal("FlexNet plugin not initialized — call initialize() first")
        })?;

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
            "FlexNet hooks registered: after_login, before_logout, \
             after_session_terminate, on_session_expired, on_session_idle"
        );

        Ok(())
    }

    /// Get the license manager (only available after initialization)
    pub fn manager(&self) -> Option<&Arc<LicenseManager>> {
        self.manager.as_ref()
    }

    /// Get plugin info
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Shutdown the plugin.
    ///
    /// Stops pool sync, releases all checkouts, and cleans up.
    pub async fn shutdown(&mut self) -> Result<(), AppError> {
        tracing::info!("Shutting down FlexNet plugin");

        // Stop pool sync
        if let Some(tx) = self.pool_sync_cancel.take() {
            let _ = tx.send(true);
        }

        // Shutdown license manager (releases all via DLL + updates DB)
        if let Some(ref manager) = self.manager {
            manager.shutdown().await?;
        }

        self.manager = None;
        tracing::info!("FlexNet plugin shut down");
        Ok(())
    }
}

impl Default for FlexNetPlugin {
    fn default() -> Self {
        Self::new()
    }
}
