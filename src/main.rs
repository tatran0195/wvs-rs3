//! FileHub Server — Enterprise File Management Platform
//!
//! Main entry point that wires all crates together and starts the server.

use std::sync::Arc;

use tokio::sync::watch;
use tracing;
use tracing_subscriber::{EnvFilter, fmt};

use filehub_core::config::AppConfig;
use filehub_core::error::AppError;

#[tokio::main]
async fn main() {
    let config = match load_configuration() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    init_logging(&config);

    if let Err(e) = run(config).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Load configuration from file and environment
fn load_configuration() -> Result<AppConfig, AppError> {
    let config_path =
        std::env::var("FILEHUB_CONFIG").unwrap_or_else(|_| "config/default.toml".to_string());

    let env = std::env::var("FILEHUB_ENV").unwrap_or_else(|_| "development".to_string());

    tracing::info!("Loading config from '{}' (env: {})", config_path, env);

    let mut config = AppConfig::load(&config_path)
        .map_err(|e| AppError::internal(format!("Config load error: {}", e)))?;

    let env_config_path = format!("config/{}.toml", env);
    if std::path::Path::new(&env_config_path).exists() {
        let env_config = AppConfig::load(&env_config_path)
            .map_err(|e| AppError::internal(format!("Env config load error: {}", e)))?;
        config.merge(env_config);
    }

    Ok(config)
}

/// Initialize tracing/logging
fn init_logging(config: &AppConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    match config.logging.format.as_str() {
        "json" => {
            fmt()
                .json()
                .with_env_filter(filter)
                .with_target(true)
                .with_thread_ids(true)
                .init();
        }
        _ => {
            fmt()
                .pretty()
                .with_env_filter(filter)
                .with_target(true)
                .init();
        }
    }
}

/// Main server run function
async fn run(config: AppConfig) -> Result<(), AppError> {
    tracing::info!("Starting FileHub v{}", env!("CARGO_PKG_VERSION"));

    // ── Step 1: Create data directories ──────────────────────────
    create_data_directories(&config).await?;

    // ── Step 2: Database connection + migrations ─────────────────
    tracing::info!("Connecting to database...");
    let db_pool = filehub_database::connection::create_pool(&config.database)
        .await
        .map_err(|e| AppError::internal(format!("Database connection failed: {}", e)))?;

    tracing::info!("Running database migrations...");
    filehub_database::migration::run_migrations(&db_pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration failed: {}", e)))?;
    tracing::info!("Database migrations complete");

    // ── Step 3: Initialize cache ─────────────────────────────────
    tracing::info!(
        "Initializing cache (provider: {})...",
        config.cache.provider
    );
    let cache = filehub_cache::provider::CacheManager::new(&config.cache)
        .await
        .map_err(|e| AppError::internal(format!("Cache init failed: {}", e)))?;
    let cache = Arc::new(cache);
    tracing::info!("Cache initialized");

    // ── Step 4: Initialize storage providers ─────────────────────
    tracing::info!("Initializing storage providers...");
    let storage_manager = filehub_storage::manager::StorageManager::new(&config.storage)
        .await
        .map_err(|e| AppError::internal(format!("Storage init failed: {}", e)))?;
    let storage_manager = Arc::new(storage_manager);
    tracing::info!("Storage providers initialized");

    // ── Step 5: Initialize repositories ──────────────────────────
    let user_repo = Arc::new(filehub_database::repositories::user::UserRepository::new(
        db_pool.clone(),
    ));
    let session_repo =
        Arc::new(filehub_database::repositories::session::SessionRepository::new(db_pool.clone()));
    let file_repo = Arc::new(filehub_database::repositories::file::FileRepository::new(
        db_pool.clone(),
    ));
    let folder_repo =
        Arc::new(filehub_database::repositories::folder::FolderRepository::new(db_pool.clone()));
    let storage_repo =
        Arc::new(filehub_database::repositories::storage::StorageRepository::new(db_pool.clone()));
    let permission_repo =
        Arc::new(filehub_database::repositories::permission::AclRepository::new(db_pool.clone()));
    let share_repo = Arc::new(filehub_database::repositories::share::ShareRepository::new(
        db_pool.clone(),
    ));
    let job_repo = Arc::new(filehub_database::repositories::job::JobRepository::new(
        db_pool.clone(),
    ));
    let notification_repo = Arc::new(
        filehub_database::repositories::notification::NotificationRepository::new(db_pool.clone()),
    );
    let audit_repo =
        Arc::new(filehub_database::repositories::audit::AuditLogRepository::new(db_pool.clone()));
    let license_repo = Arc::new(
        filehub_database::repositories::license::LicenseCheckoutRepository::new(db_pool.clone()),
    );
    let snapshot_repo = Arc::new(
        filehub_database::repositories::pool_snapshot::PoolSnapshotRepository::new(db_pool.clone()),
    );

    // ── Step 6: Initialize auth system ───────────────────────────
    tracing::info!("Initializing authentication system...");
    let password_hasher = Arc::new(filehub_auth::password::hasher::PasswordHasher::new(
        &config.auth,
    ));
    let jwt_encoder = Arc::new(filehub_auth::jwt::encoder::JwtEncoder::new(&config.auth));
    let jwt_decoder = Arc::new(filehub_auth::jwt::decoder::JwtDecoder::new(
        &config.auth,
        Arc::clone(&cache),
    ));
    let session_manager = Arc::new(filehub_auth::session::manager::SessionManager::new(
        Arc::clone(&session_repo),
        Arc::clone(&cache),
        config.session.clone(),
    ));
    let rbac_enforcer = Arc::new(filehub_auth::rbac::enforcer::RbacEnforcer::new());
    let acl_checker = Arc::new(filehub_auth::acl::checker::AclChecker::new(
        Arc::clone(&permission_repo),
        Arc::clone(&cache),
    ));
    let permission_resolver = Arc::new(
        filehub_auth::acl::resolver::EffectivePermissionResolver::new(
            Arc::clone(&rbac_enforcer),
            Arc::clone(&acl_checker),
            Arc::clone(&share_repo),
            Arc::clone(&cache),
        ),
    );

    // ── Step 7: Initialize seat allocator ────────────────────────
    tracing::info!("Initializing seat allocator...");
    let seat_allocator = Arc::new(filehub_auth::seat::allocator::SeatAllocatorDispatch::new(
        &config.session,
        Arc::clone(&cache),
        Arc::clone(&session_repo),
    ));
    tracing::info!("Seat allocator initialized");

    // ── Step 8: Initialize plugin manager ────────────────────────
    tracing::info!("Initializing plugin system...");
    let mut plugin_manager = filehub_plugin::manager::PluginManager::new();
    let mut hook_registry = filehub_plugin::hooks::registry::HookRegistry::new();

    // ── Step 8a: FlexNet plugin ──────────────────────────────────
    let license_manager = if config.license.enabled {
        tracing::info!("Loading FlexNet license plugin...");

        #[cfg(feature = "mock")]
        let bindings: Arc<dyn plugin_flexnet::ffi::bindings::FlexNetBindings> = {
            let mock = plugin_flexnet::ffi::bindings::mock::MockFlexNetBindings::new();
            mock.set_total_seats(&config.license.feature_name, 10);
            Arc::new(mock)
        };

        #[cfg(not(feature = "mock"))]
        let bindings: Arc<dyn plugin_flexnet::ffi::bindings::FlexNetBindings> =
            { Arc::new(plugin_flexnet::ffi::bindings::mock::MockFlexNetBindings::new()) };

        let mut flexnet_plugin = plugin_flexnet::FlexNetPlugin::new();
        let manager = flexnet_plugin
            .initialize(
                config.license.clone(),
                bindings,
                Arc::clone(&license_repo),
                Arc::clone(&snapshot_repo),
            )
            .await?;

        flexnet_plugin.register_hooks(&mut hook_registry)?;
        tracing::info!("FlexNet plugin loaded");
        Some(manager)
    } else {
        tracing::info!("License system disabled");
        None
    };

    // ── Step 8b: CAD converter plugin ────────────────────────────
    tracing::info!("Loading CAD converter plugin...");
    let mut cad_plugin = plugin_cad_converter::CadConverterPlugin::new();
    let cad_converter = cad_plugin
        .initialize(
            std::path::PathBuf::from(&config.storage.data_root).join("temp"),
            std::path::PathBuf::from(&config.storage.data_root).join("cache/conversions"),
            None,
        )
        .await?;
    cad_plugin.register_hooks(&mut hook_registry)?;
    tracing::info!("CAD converter plugin loaded");

    let hook_registry = Arc::new(hook_registry);

    // ── Step 9: Initialize services ──────────────────────────────
    tracing::info!("Initializing services...");
    let file_service = Arc::new(filehub_service::file::service::FileService::new(
        Arc::clone(&file_repo),
        Arc::clone(&folder_repo),
        Arc::clone(&storage_manager),
        Arc::clone(&permission_resolver),
        Arc::clone(&cache),
    ));
    let upload_service = Arc::new(filehub_service::file::upload::UploadService::new(
        Arc::clone(&file_repo),
        Arc::clone(&folder_repo),
        Arc::clone(&storage_manager),
        Arc::clone(&cache),
    ));
    let folder_service = Arc::new(filehub_service::folder::service::FolderService::new(
        Arc::clone(&folder_repo),
        Arc::clone(&storage_repo),
        Arc::clone(&permission_resolver),
        Arc::clone(&cache),
    ));
    let share_service = Arc::new(filehub_service::share::service::ShareService::new(
        Arc::clone(&share_repo),
        Arc::clone(&file_repo),
        Arc::clone(&folder_repo),
        Arc::clone(&permission_resolver),
        Arc::clone(&cache),
    ));
    let notification_service = Arc::new(
        filehub_service::notification::service::NotificationService::new(
            Arc::clone(&notification_repo),
            Arc::clone(&cache),
        ),
    );
    let storage_service = Arc::new(filehub_service::storage::service::StorageService::new(
        Arc::clone(&storage_repo),
        Arc::clone(&storage_manager),
    ));
    let permission_service = Arc::new(
        filehub_service::permission::service::PermissionService::new(
            Arc::clone(&permission_repo),
            Arc::clone(&cache),
        ),
    );
    let session_service = Arc::new(filehub_service::session::service::SessionService::new(
        Arc::clone(&session_repo),
        Arc::clone(&cache),
    ));

    tracing::info!("Services initialized");

    // ── Step 10: Initialize realtime engine ───────────────────────
    tracing::info!("Initializing realtime engine...");
    let realtime_engine = Arc::new(
        filehub_realtime::server::RealtimeEngine::new(
            &config.realtime,
            Arc::clone(&jwt_decoder),
            Arc::clone(&session_repo),
            Arc::clone(&notification_service),
        )
        .await,
    );
    tracing::info!("Realtime engine initialized");

    // ── Step 11: Shutdown channel ────────────────────────────────
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // ── Step 12: Start background worker ─────────────────────────
    let worker_handle = if config.worker.enabled {
        tracing::info!("Starting background worker...");

        let worker_id = format!("worker-{}", &uuid::Uuid::new_v4().to_string()[..8]);

        let job_queue = Arc::new(filehub_worker::queue::JobQueue::new(
            Arc::clone(&job_repo),
            worker_id.clone(),
        ));

        let mut job_executor = filehub_worker::executor::JobExecutor::new();

        // Register job handlers
        let cleanup_handler = Arc::new(filehub_worker::jobs::cleanup::CleanupJobHandler::new(
            Arc::clone(&session_repo),
            Arc::clone(&file_repo),
            std::path::PathBuf::from(&config.storage.data_root),
        ));

        job_executor.register(Arc::new(
            filehub_worker::jobs::cleanup::SessionCleanupHandler::new(Arc::clone(&cleanup_handler)),
        ));
        job_executor.register(Arc::new(
            filehub_worker::jobs::cleanup::ChunkCleanupHandler::new(Arc::clone(&cleanup_handler)),
        ));
        job_executor.register(Arc::new(
            filehub_worker::jobs::cleanup::TempCleanupHandler::new(Arc::clone(&cleanup_handler)),
        ));
        job_executor.register(Arc::new(
            filehub_worker::jobs::cleanup::VersionCleanupHandler::new(Arc::clone(&cleanup_handler)),
        ));

        let report_handler = Arc::new(filehub_worker::jobs::report::ReportJobHandler::new(
            Arc::clone(&user_repo),
            Arc::clone(&file_repo),
            Arc::clone(&storage_repo),
            Arc::clone(&session_repo),
            Arc::clone(&audit_repo),
        ));
        job_executor.register(report_handler);

        let maintenance_handler = Arc::new(
            filehub_worker::jobs::maintenance::MaintenanceJobHandler::new(
                Arc::clone(&file_repo),
                Arc::clone(&storage_repo),
            ),
        );
        job_executor.register(maintenance_handler);

        let notification_handler = Arc::new(
            filehub_worker::jobs::notification::NotificationJobHandler::new(
                Arc::clone(&notification_repo),
                config.realtime.notifications.cleanup_after_days as i64,
                config.realtime.notifications.max_stored_per_user as i64,
            ),
        );
        job_executor.register(notification_handler);

        let license_handler = Arc::new(filehub_worker::jobs::license::LicenseJobHandler::new(None));
        job_executor.register(license_handler);

        let presence_handler = Arc::new(filehub_worker::jobs::presence::PresenceJobHandler::new(
            Arc::clone(&session_repo),
            None,
            config.session.heartbeat_timeout_seconds as i64,
        ));
        job_executor.register(presence_handler);

        let idle_handler = Arc::new(
            filehub_worker::jobs::presence::IdleSessionCheckHandler::new(
                Arc::clone(&session_repo),
                config.session.idle_timeout_minutes as i64,
            ),
        );
        job_executor.register(idle_handler);

        let job_executor = Arc::new(job_executor);
        let worker_runner = filehub_worker::runner::WorkerRunner::new(
            Arc::clone(&job_queue),
            Arc::clone(&job_executor),
            config.worker.clone(),
            worker_id,
        );

        // Start cron scheduler
        let scheduler = filehub_worker::scheduler::CronScheduler::new(Arc::clone(&job_queue))
            .await
            .map_err(|e| AppError::internal(format!("Scheduler init failed: {}", e)))?;
        scheduler.register_default_tasks().await?;
        scheduler.start().await?;

        let worker_cancel = shutdown_rx.clone();
        let handle = tokio::spawn(async move {
            worker_runner.run(worker_cancel).await;
        });

        tracing::info!("Background worker started");
        Some(handle)
    } else {
        tracing::info!("Background worker disabled");
        None
    };

    // ── Step 13: Start WebDAV server ─────────────────────────────
    let webdav_handle = if config.storage.webdav_server.enabled {
        tracing::info!(
            "Starting WebDAV server on port {}...",
            config.storage.webdav_server.port
        );

        let webdav_server = filehub_webdav::WebDavServer::new(
            config.storage.webdav_server.clone(),
            Arc::clone(&file_service),
            Arc::clone(&upload_service),
            Arc::clone(&folder_service),
            Arc::clone(&user_repo),
            Arc::clone(&password_hasher),
        );

        let webdav_cancel = shutdown_rx.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = webdav_server.start(webdav_cancel).await {
                tracing::error!("WebDAV server error: {}", e);
            }
        });

        tracing::info!("WebDAV server started");
        Some(handle)
    } else {
        tracing::info!("WebDAV server disabled");
        None
    };

    // ── Step 14: Build and start HTTP server ─────────────────────
    tracing::info!(
        "Starting HTTP server on {}:{}...",
        config.server.host,
        config.server.port
    );

    let app_state = filehub_api::state::AppState {
        // Configuration
        config: Arc::new(config.clone()),

        // Infrastructure
        db_pool: db_pool.clone(),
        cache: Arc::clone(&cache),
        storage_manager: Arc::clone(&storage_manager),

        // Auth
        jwt_encoder: Arc::clone(&jwt_encoder),
        jwt_decoder: Arc::clone(&jwt_decoder),
        password_hasher: Arc::clone(&password_hasher),
        session_manager: Arc::clone(&session_manager),
        seat_allocator: Arc::clone(&seat_allocator),
        rbac_enforcer: Arc::clone(&rbac_enforcer),
        permission_resolver: Arc::clone(&permission_resolver),

        // Plugins & Realtime
        hook_registry: Arc::clone(&hook_registry),
        realtime_engine: Arc::clone(&realtime_engine),

        // Repositories
        user_repo: Arc::clone(&user_repo),
        session_repo: Arc::clone(&session_repo),
        file_repo: Arc::clone(&file_repo),
        folder_repo: Arc::clone(&folder_repo),
        storage_repo: Arc::clone(&storage_repo),
        share_repo: Arc::clone(&share_repo),
        permission_repo: Arc::clone(&permission_repo),
        notification_repo: Arc::clone(&notification_repo),
        audit_repo: Arc::clone(&audit_repo),
        job_repo: Arc::clone(&job_repo),
        license_repo: Arc::clone(&license_repo),
        snapshot_repo: Arc::clone(&snapshot_repo),

        // Services
        file_service: Arc::clone(&file_service),
        upload_service: Arc::clone(&upload_service),
        folder_service: Arc::clone(&folder_service),
        share_service: Arc::clone(&share_service),
        notification_service: Arc::clone(&notification_service),
        storage_service: Arc::clone(&storage_service),
        permission_service: Arc::clone(&permission_service),
        session_service: Arc::clone(&session_service),
    };

    let app = filehub_api::router::build_router(app_state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| AppError::internal(format!("Failed to bind {}: {}", addr, e)))?;

    tracing::info!("FileHub server listening on {}", addr);

    // ── Step 15: Graceful shutdown ───────────────────────────────
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        tracing::info!("Shutdown signal received, starting graceful shutdown...");
        let _ = shutdown_tx.send(true);
    });

    server
        .await
        .map_err(|e| AppError::internal(format!("Server error: {}", e)))?;

    // ── Step 16: Wait for background tasks ───────────────────────
    tracing::info!("Waiting for background tasks to complete...");

    if let Some(handle) = worker_handle {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(30), handle).await;
    }
    if let Some(handle) = webdav_handle {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), handle).await;
    }

    tracing::info!("FileHub server shut down gracefully");
    Ok(())
}

/// Create required data directories
async fn create_data_directories(config: &AppConfig) -> Result<(), AppError> {
    let dirs = [
        format!("{}/storage/local", config.storage.data_root),
        format!("{}/cache/thumbnails", config.storage.data_root),
        format!("{}/cache/conversions", config.storage.data_root),
        format!("{}/temp", config.storage.data_root),
        format!("{}/logs", config.storage.data_root),
        format!("{}/plugins", config.storage.data_root),
        format!("{}/backups", config.storage.data_root),
    ];

    for dir in &dirs {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create dir '{}': {}", dir, e)))?;
    }

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
