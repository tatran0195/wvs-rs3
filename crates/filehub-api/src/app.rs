//! Application builder — wires router + middleware + state into an Axum app.

use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::watch;
use tower_http::trace::TraceLayer;

use filehub_core::config::AppConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::{
    audit, file, folder, job, license, notification, permission, pool_snapshot, session,
    session_limit, share, storage, user,
};
use filehub_worker::jobs::cleanup::{
    ChunkCleanupHandler, SessionCleanupHandler, TempCleanupHandler, VersionCleanupHandler,
};

use crate::middleware::compression::build_compression_layer;
use crate::middleware::cors::build_cors_layer;
use crate::router::build_router;
use crate::state::AppState;

/// Builds the complete Axum application with all routes and middleware.
pub fn build_app(state: AppState, cors_config: &filehub_core::config::app::CorsConfig) -> Router {
    build_router(state)
        .layer(build_compression_layer())
        .layer(build_cors_layer(cors_config))
        .layer(TraceLayer::new_for_http())
}

/// Runs the FileHub server with the given configuration and database pool.
pub async fn run_server(config: AppConfig, db_pool: PgPool) -> Result<(), AppError> {
    tracing::info!("Starting FileHub server...");

    // ── Step 1: Create data directories ──────────────────────────
    create_data_directories(&config).await?;

    // ── Step 2: Initialize cache ─────────────────────────────────
    tracing::info!(
        "Initializing cache (provider: {})...",
        config.cache.provider
    );
    let cache = filehub_cache::provider::CacheManager::new(&config.cache)
        .await
        .map_err(|e| AppError::internal(format!("Cache init failed: {}", e)))?;
    let cache = Arc::new(cache);

    // ── Step 3: Initialize storage providers ─────────────────────
    let storage_manager = Arc::new(filehub_storage::manager::StorageManager::new());

    // ── Step 4: Initialize repositories ──────────────────────────
    let user_repo = Arc::new(user::UserRepository::new(db_pool.clone()));
    let session_repo = Arc::new(session::SessionRepository::new(db_pool.clone()));
    let file_repo = Arc::new(file::FileRepository::new(db_pool.clone()));
    let folder_repo = Arc::new(folder::FolderRepository::new(db_pool.clone()));
    let storage_repo = Arc::new(storage::StorageRepository::new(db_pool.clone()));
    let permission_repo = Arc::new(permission::AclRepository::new(db_pool.clone()));
    let share_repo = Arc::new(share::ShareRepository::new(db_pool.clone()));
    let job_repo = Arc::new(job::JobRepository::new(db_pool.clone()));
    let notification_repo = Arc::new(notification::NotificationRepository::new(db_pool.clone()));
    let audit_repo = Arc::new(audit::AuditLogRepository::new(db_pool.clone()));
    let license_repo = Arc::new(license::LicenseCheckoutRepository::new(db_pool.clone()));
    let snapshot_repo = Arc::new(pool_snapshot::PoolSnapshotRepository::new(db_pool.clone()));
    let session_limit_repo = Arc::new(session_limit::SessionLimitRepository::new(db_pool.clone()));

    // ── Step 5: Initialize auth system ───────────────────────────
    let password_hasher = Arc::new(filehub_auth::password::hasher::PasswordHasher::new());
    let jwt_encoder = Arc::new(filehub_auth::jwt::encoder::JwtEncoder::new(&config.auth));
    let jwt_decoder = Arc::new(filehub_auth::jwt::decoder::JwtDecoder::new(
        &config.auth,
        Arc::clone(&cache),
    ));

    let session_store = Arc::new(filehub_auth::session::store::SessionStore::new(
        Arc::clone(&session_repo),
        config.session.clone(),
    ));

    let session_limiter = Arc::new(filehub_auth::SessionLimiter::new(
        Arc::clone(&session_limit_repo),
        config.session.clone(),
    ));

    let seat_allocator = Arc::new(filehub_auth::seat::allocator::SeatAllocatorDispatch::new(
        &config.session,
        Arc::clone(&cache),
        Arc::clone(&session_repo),
    ));

    let session_manager = Arc::new(filehub_auth::session::manager::SessionManager::new(
        Arc::clone(&jwt_encoder),
        Arc::clone(&jwt_decoder),
        Arc::clone(&session_store),
        Arc::clone(&user_repo),
        Arc::clone(&password_hasher),
        Arc::clone(&seat_allocator) as Arc<dyn filehub_auth::SeatAllocator>,
        Arc::clone(&session_limiter),
        Arc::clone(&cache),
        config.auth.clone(),
        config.session.clone(),
    ));

    let rbac_enforcer = Arc::new(filehub_auth::rbac::enforcer::RbacEnforcer::new());
    let acl_checker = Arc::new(filehub_auth::acl::checker::AclChecker::new(Arc::clone(
        &permission_repo,
    )));
    let inheritance_resolver =
        Arc::new(filehub_auth::acl::inheritance::AclInheritanceResolver::new(
            Arc::clone(&folder_repo),
            Arc::clone(&permission_repo),
        ));
    let password_validator = Arc::new(filehub_auth::password::validator::PasswordValidator::new(
        &config.auth,
    ));
    let permission_resolver = Arc::new(
        filehub_auth::acl::resolver::EffectivePermissionResolver::new(
            Arc::clone(&rbac_enforcer),
            Arc::clone(&acl_checker),
            Arc::clone(&inheritance_resolver),
            Arc::clone(&cache),
        ),
    );

    // ── Step 6: Initialize plugin manager ────────────────────────
    let plugin_manager = Arc::new(filehub_plugin::manager::PluginManager::new());

    if config.license.enabled {
        let dll_path = if config.license.license_file.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from("license_proxy.dll"))
        };

        let flexnet_plugin = plugin_flexnet::FlexNetPlugin::new();
        flexnet_plugin
            .initialize(
                config.license.clone(),
                dll_path,
                Arc::clone(&license_repo),
                Arc::clone(&snapshot_repo),
            )
            .await?;

        flexnet_plugin
            .register_hooks(plugin_manager.hook_registry())
            .await?;

        plugin_manager
            .plugin_registry()
            .register(Arc::new(flexnet_plugin))
            .await
            .map_err(|e| AppError::internal(format!("Failed to register FlexNet: {}", e)))?;
    }

    if config.storage.conversions.enabled {
        let cad_plugin = Arc::new(plugin_cad_converter::CadConverterPlugin::new());
        cad_plugin.initialize().await?;
        cad_plugin
            .register_hooks(plugin_manager.hook_registry())
            .await;

        plugin_manager
            .plugin_registry()
            .register(cad_plugin)
            .await
            .map_err(|e| AppError::internal(format!("Failed to register CAD converter: {}", e)))?;
    }

    // ── Step 7: Initialize services ──────────────────────────────
    let file_service = Arc::new(filehub_service::file::service::FileService::new(
        Arc::clone(&file_repo),
        Arc::clone(&folder_repo),
        Arc::clone(&permission_resolver),
    ));
    let upload_service = Arc::new(filehub_service::file::upload::UploadService::new(
        Arc::clone(&file_repo),
        Arc::clone(&folder_repo),
        Arc::clone(&storage_manager),
        Arc::clone(&permission_resolver),
        config.storage.clone(),
        Arc::clone(&plugin_manager),
    ));
    let folder_service = Arc::new(filehub_service::folder::service::FolderService::new(
        Arc::clone(&folder_repo),
        Arc::clone(&storage_repo),
        Arc::clone(&permission_resolver),
    ));
    let link_service = Arc::new(filehub_service::share::LinkService::new());
    let share_service = Arc::new(filehub_service::share::service::ShareService::new(
        Arc::clone(&share_repo),
        Arc::clone(&link_service),
        Arc::clone(&password_hasher),
    ));
    let notification_service = Arc::new(
        filehub_service::notification::service::NotificationService::new(Arc::clone(
            &notification_repo,
        )),
    );
    let storage_service = Arc::new(filehub_service::storage::service::StorageService::new(
        Arc::clone(&storage_repo),
        Arc::clone(&rbac_enforcer),
    ));
    let permission_service = Arc::new(
        filehub_service::permission::service::PermissionService::new(
            Arc::clone(&permission_repo),
            Arc::clone(&rbac_enforcer),
            Arc::clone(&permission_resolver),
        ),
    );
    let session_service = Arc::new(filehub_service::session::service::SessionService::new(
        Arc::clone(&session_store),
        Arc::clone(&rbac_enforcer),
    ));

    let access_service = Arc::new(filehub_service::share::AccessService::new(
        Arc::clone(&share_repo),
        Arc::clone(&password_hasher),
    ));
    let admin_user_service = Arc::new(filehub_service::user::AdminUserService::new(
        Arc::clone(&user_repo),
        Arc::clone(&password_hasher),
        Arc::clone(&password_validator),
        Arc::clone(&rbac_enforcer),
    ));
    let user_service = Arc::new(filehub_service::user::UserService::new(
        Arc::clone(&user_repo),
        Arc::clone(&password_hasher),
        Arc::clone(&password_validator),
    ));
    let report_service = Arc::new(filehub_service::report::WeeklyReportService::new(
        Arc::clone(&user_repo),
        Arc::clone(&file_repo),
        Arc::clone(&audit_repo),
    ));
    let download_service = Arc::new(filehub_service::file::DownloadService::new(
        Arc::clone(&file_repo),
        Arc::clone(&storage_manager),
        Arc::clone(&permission_resolver),
    ));
    let preview_service = Arc::new(filehub_service::file::PreviewService::new(
        Arc::clone(&file_repo),
        Arc::clone(&storage_manager),
        Arc::clone(&permission_resolver),
        Arc::clone(&cache),
    ));
    let search_service = Arc::new(filehub_service::file::SearchService::new(Arc::clone(
        &file_repo,
    )));
    let version_service = Arc::new(filehub_service::file::VersionService::new(
        Arc::clone(&file_repo),
        Arc::clone(&permission_resolver),
    ));
    let tree_service = Arc::new(filehub_service::folder::TreeService::new(Arc::clone(
        &folder_repo,
    )));
    let termination_service = Arc::new(filehub_service::session::TerminationService::new(
        Arc::clone(&session_manager),
        Arc::clone(&rbac_enforcer),
    ));
    let audit_service = Arc::new(filehub_service::session::SessionAudit::new(Arc::clone(
        &audit_repo,
    )));

    // ── Step 8: Initialize realtime engine ───────────────────────
    let realtime_engine = Arc::new(
        filehub_realtime::server::RealtimeEngine::new(
            &config.realtime,
            Arc::clone(&jwt_decoder),
            Arc::clone(&session_repo),
            Arc::clone(&notification_service),
        )
        .await,
    );

    // ── Step 9: Shutdown channel & worker ────────────────────────
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let _worker_handle = if config.worker.enabled {
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

        job_executor.register(Arc::new(SessionCleanupHandler::new(Arc::clone(
            &cleanup_handler,
        ))));
        job_executor.register(Arc::new(ChunkCleanupHandler::new(Arc::clone(
            &cleanup_handler,
        ))));
        job_executor.register(Arc::new(TempCleanupHandler::new(Arc::clone(
            &cleanup_handler,
        ))));
        job_executor.register(Arc::new(VersionCleanupHandler::new(Arc::clone(
            &cleanup_handler,
        ))));

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

        let worker_cancel = shutdown_rx.clone();
        Some(tokio::spawn(async move {
            worker_runner.run(worker_cancel).await;
        }))
    } else {
        None
    };

    // ── Step 10: Build and start HTTP server ─────────────────────
    let app_state = AppState {
        config: Arc::new(config.clone()),
        db_pool: db_pool.clone(),
        cache,
        storage_manager,
        jwt_encoder,
        jwt_decoder,
        password_hasher,
        session_manager,
        seat_allocator,
        rbac_enforcer,
        permission_resolver,
        plugin_manager,
        realtime: realtime_engine,
        user_repo,
        session_repo,
        file_repo,
        folder_repo,
        storage_repo,
        share_repo,
        permission_repo,
        notification_repo,
        audit_repo,
        job_repo,
        license_repo,
        snapshot_repo,
        file_service,
        upload_service,
        folder_service,
        share_service,
        notification_service,
        storage_service,
        permission_service,
        session_service,
        audit_service,
        admin_user_service,
        user_service,
        report_service,
        download_service,
        preview_service,
        version_service,
        tree_service,
        termination_service,
        search_service,
        access_service,
    };

    let app = build_app(app_state, &config.server.cors);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| AppError::internal(format!("Failed to bind {}: {}", addr, e)))?;

    tracing::info!("FileHub server listening on {}", addr);

    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });

    server
        .await
        .map_err(|e| AppError::internal(format!("Server error: {}", e)))?;

    Ok(())
}

async fn create_data_directories(config: &AppConfig) -> Result<(), AppError> {
    let dirs = [
        format!("{}/storage/local", config.storage.data_root),
        format!("{}/cache/thumbnails", config.storage.data_root),
        format!("{}/cache/conversions", config.storage.data_root),
        format!("{}/temp", config.storage.data_root),
        format!("{}/logs", config.storage.data_root),
        format!("{}/plugins", config.storage.data_root),
    ];

    for dir in &dirs {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create dir '{}': {}", dir, e)))?;
    }

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
}
