//! Application state shared across all handlers and middleware.

use std::sync::Arc;

use sqlx::PgPool;

use filehub_auth::acl::resolver::EffectivePermissionResolver;
use filehub_auth::jwt::decoder::JwtDecoder;
use filehub_auth::jwt::encoder::JwtEncoder;
use filehub_auth::password::hasher::PasswordHasher;
use filehub_auth::rbac::enforcer::RbacEnforcer;
use filehub_auth::seat::allocator::SeatAllocatorDispatch;
use filehub_auth::session::manager::SessionManager;
use filehub_cache::provider::CacheManager;
use filehub_core::config::AppConfig;
use filehub_plugin::hooks::registry::HookRegistry;
use filehub_realtime::server::RealtimeEngine;
use filehub_storage::manager::StorageManager;

use filehub_database::repositories::audit::AuditLogRepository;
use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::folder::FolderRepository;
use filehub_database::repositories::job::JobRepository;
use filehub_database::repositories::license::LicenseCheckoutRepository;
use filehub_database::repositories::notification::NotificationRepository;
use filehub_database::repositories::permission::AclRepository;
use filehub_database::repositories::pool_snapshot::PoolSnapshotRepository;
use filehub_database::repositories::session::SessionRepository;
use filehub_database::repositories::share::ShareRepository;
use filehub_database::repositories::storage::StorageRepository;
use filehub_database::repositories::user::UserRepository;

use filehub_service::file::service::FileService;
use filehub_service::file::upload::UploadService;
use filehub_service::folder::service::FolderService;
use filehub_service::notification::service::NotificationService;
use filehub_service::permission::service::PermissionService;
use filehub_service::session::service::SessionService;
use filehub_service::share::service::ShareService;
use filehub_service::storage::service::StorageService;

/// Application state containing all shared dependencies.
///
/// Passed to every Axum handler via `State<AppState>`.
/// All fields are `Arc`-wrapped for cheap cloning across tasks.
#[derive(Debug, Clone)]
pub struct AppState {
    // ── Configuration ────────────────────────────────────────
    /// Application configuration
    pub config: Arc<AppConfig>,

    // ── Infrastructure ───────────────────────────────────────
    /// PostgreSQL connection pool
    pub db_pool: PgPool,
    /// Cache manager (Redis or in-memory)
    pub cache: Arc<CacheManager>,
    /// Storage provider manager
    pub storage_manager: Arc<StorageManager>,

    // ── Auth ─────────────────────────────────────────────────
    /// JWT token encoder
    pub jwt_encoder: Arc<JwtEncoder>,
    /// JWT token decoder and validator
    pub jwt_decoder: Arc<JwtDecoder>,
    /// Password hasher (Argon2)
    pub password_hasher: Arc<PasswordHasher>,
    /// Session lifecycle manager
    pub session_manager: Arc<SessionManager>,
    /// Seat allocation dispatcher
    pub seat_allocator: Arc<SeatAllocatorDispatch>,
    /// Role-based access control enforcer
    pub rbac_enforcer: Arc<RbacEnforcer>,
    /// Effective permission resolver (RBAC + ACL + Share)
    pub permission_resolver: Arc<EffectivePermissionResolver>,

    // ── Plugins & Realtime ───────────────────────────────────
    /// Plugin hook registry
    pub hook_registry: Arc<HookRegistry>,
    /// WebSocket realtime engine
    pub realtime_engine: Arc<RealtimeEngine>,

    // ── Repositories ─────────────────────────────────────────
    /// User repository
    pub user_repo: Arc<UserRepository>,
    /// Session repository
    pub session_repo: Arc<SessionRepository>,
    /// File repository
    pub file_repo: Arc<FileRepository>,
    /// Folder repository
    pub folder_repo: Arc<FolderRepository>,
    /// Storage repository
    pub storage_repo: Arc<StorageRepository>,
    /// Share repository
    pub share_repo: Arc<ShareRepository>,
    /// ACL/Permission repository
    pub permission_repo: Arc<AclRepository>,
    /// Notification repository
    pub notification_repo: Arc<NotificationRepository>,
    /// Audit log repository
    pub audit_repo: Arc<AuditLogRepository>,
    /// Job repository
    pub job_repo: Arc<JobRepository>,
    /// License checkout repository
    pub license_repo: Arc<LicenseCheckoutRepository>,
    /// Pool snapshot repository
    pub snapshot_repo: Arc<PoolSnapshotRepository>,

    // ── Services ─────────────────────────────────────────────
    /// File service
    pub file_service: Arc<FileService>,
    /// Upload service
    pub upload_service: Arc<UploadService>,
    /// Folder service
    pub folder_service: Arc<FolderService>,
    /// Share service
    pub share_service: Arc<ShareService>,
    /// Notification service
    pub notification_service: Arc<NotificationService>,
    /// Storage management service
    pub storage_service: Arc<StorageService>,
    /// Permission management service
    pub permission_service: Arc<PermissionService>,
    /// Session management service
    pub session_service: Arc<SessionService>,
}
