//! FlexNet plugin — registers with the FileHub plugin system.

use std::path::PathBuf;
use std::sync::Arc;

use filehub_core::config::LicenseConfig;
use filehub_plugin::HookPoint;
use tracing;

use filehub_core::error::AppError;
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_plugin::hooks::registry::HookRegistry;

use crate::ffi::wrapper::LicenseManagerWrapper;
use crate::hooks::{
    AfterLoginHook, AfterSessionTerminateHook, BeforeLogoutHook, OnSessionExpiredHook,
    OnSessionIdleHook,
};
use crate::license::manager::LicenseManager;
use crate::license::pool::PoolSyncService;

/// Plugin name used for registration, logging, and hook results.
const PLUGIN_NAME: &str = "flexnet";

/// Plugin version from Cargo manifest.
const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// FlexNet license plugin for FileHub
#[derive(Debug)]
pub struct FlexNetPlugin {
    /// License manager (set after initialization)
    manager: Arc<tokio::sync::RwLock<Option<Arc<LicenseManager>>>>,
    /// Pool sync cancellation sender
    pool_sync_cancel: Arc<tokio::sync::RwLock<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl FlexNetPlugin {
    /// Create a new FlexNet plugin instance
    pub fn new() -> Self {
        Self {
            manager: Arc::new(tokio::sync::RwLock::new(None)),
            pool_sync_cancel: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
}

impl FlexNetPlugin {
    /// Initialize the plugin.
    ///
    /// Loads the DLL (or mock), creates the license manager, starts pool sync,
    /// and returns the license manager for use by other components.
    pub async fn initialize(
        &self,
        config: LicenseConfig,
        dll_path: Option<PathBuf>,
        checkout_repo: Arc<LicenseCheckoutRepository>,
        snapshot_repo: Arc<PoolSnapshotRepository>,
    ) -> Result<Arc<LicenseManager>, AppError> {
        tracing::info!(
            plugin = PLUGIN_NAME,
            version = PLUGIN_VERSION,
            "Initializing FlexNet plugin"
        );

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

        let mut cancel = self.pool_sync_cancel.write().await;
        *cancel = Some(tx);
        let mut manager_lock = self.manager.write().await;
        *manager_lock = Some(Arc::clone(&manager));

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
    pub async fn register_hooks(&self, registry: &HookRegistry) -> Result<(), AppError> {
        let manager_lock = self.manager.read().await;
        let manager = manager_lock.as_ref().ok_or_else(|| {
            AppError::internal("FlexNet plugin not initialized — call initialize() first")
        })?;

        registry
            .register(
                HookPoint::AfterLogin,
                Arc::new(AfterLoginHook::new(Arc::clone(manager))),
            )
            .await;

        registry
            .register(
                HookPoint::BeforeLogout,
                Arc::new(BeforeLogoutHook::new(Arc::clone(manager))),
            )
            .await;

        registry
            .register(
                HookPoint::AfterSessionTerminate,
                Arc::new(AfterSessionTerminateHook::new(Arc::clone(manager))),
            )
            .await;

        registry
            .register(
                HookPoint::OnSessionExpired,
                Arc::new(OnSessionExpiredHook::new(Arc::clone(manager))),
            )
            .await;

        registry
            .register(
                HookPoint::OnSessionIdle,
                Arc::new(OnSessionIdleHook::new(Arc::clone(manager), true)),
            )
            .await;

        tracing::info!(
            "FlexNet hooks registered: after_login, before_logout, \
             after_session_terminate, on_session_expired, on_session_idle"
        );

        Ok(())
    }

    /// Get the license manager (only available after initialization)
    pub async fn manager(&self) -> Option<Arc<LicenseManager>> {
        self.manager.read().await.clone()
    }

    /// Shutdown the plugin.
    ///
    /// Stops pool sync, releases all checkouts, and cleans up.
    pub async fn shutdown(&self) -> Result<(), AppError> {
        tracing::info!("Shutting down FlexNet plugin");

        // Stop pool sync
        let mut cancel_lock = self.pool_sync_cancel.write().await;
        if let Some(tx) = cancel_lock.take() {
            let _ = tx.send(true);
        }

        // Shutdown license manager (releases all via DLL + updates DB)
        let mut manager_lock = self.manager.write().await;
        if let Some(ref manager) = *manager_lock {
            manager.shutdown().await?;
        }

        *manager_lock = None;
        tracing::info!("FlexNet plugin shut down");
        Ok(())
    }
}

#[async_trait::async_trait]
impl filehub_plugin::registry::Plugin for FlexNetPlugin {
    fn info(&self) -> filehub_plugin::registry::PluginInfo {
        filehub_plugin::registry::PluginInfo {
            id: "flexnet".to_string(),
            name: PLUGIN_NAME.to_string(),
            version: "v1.0.0".to_string(),
            description: "FlexNet license management plugin".to_string(),
            author: "TechnoStar".to_string(),
            hooks: self.registered_hooks(),
            enabled: true,
            priority: 0,
        }
    }

    async fn on_load(&self) -> Result<(), String> {
        tracing::info!(plugin = PLUGIN_NAME, "Plugin loaded");
        Ok(())
    }

    async fn on_start(&self) -> Result<(), String> {
        tracing::info!(plugin = PLUGIN_NAME, "Plugin started");
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), String> {
        tracing::info!(plugin = PLUGIN_NAME, "Plugin stopped");
        Ok(())
    }

    async fn on_unload(&self) -> Result<(), String> {
        tracing::info!(plugin = PLUGIN_NAME, "Plugin unloaded");
        Ok(())
    }

    fn registered_hooks(&self) -> Vec<HookPoint> {
        vec![
            HookPoint::AfterLogin,
            HookPoint::BeforeLogout,
            HookPoint::AfterSessionTerminate,
            HookPoint::OnSessionExpired,
            HookPoint::OnSessionIdle,
        ]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
