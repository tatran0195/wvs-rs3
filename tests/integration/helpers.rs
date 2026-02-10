//! Shared test helpers for integration tests.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use filehub_core::config::AppConfig;
use filehub_core::error::AppError;

/// Test application context
pub struct TestApp {
    /// The Axum router for making test requests
    pub router: Router,
    /// Database pool for direct queries
    pub db_pool: PgPool,
    /// Application config
    pub config: AppConfig,
}

impl TestApp {
    /// Create a new test application
    pub async fn new() -> Self {
        let config =
            AppConfig::load("tests/fixtures/test_config.toml").expect("Failed to load test config");

        let db_pool = filehub_database::connection::create_pool(&config.database)
            .await
            .expect("Failed to connect to test database");

        filehub_database::migration::run_migrations(&db_pool)
            .await
            .expect("Failed to run migrations");

        Self::clean_database(&db_pool).await;

        let cache = Arc::new(
            filehub_cache::provider::CacheManager::new(&config.cache)
                .await
                .expect("Failed to init cache"),
        );

        let storage_manager = Arc::new(
            filehub_storage::manager::StorageManager::new(&config.storage)
                .await
                .expect("Failed to init storage"),
        );

        let user_repo = Arc::new(filehub_database::repositories::user::UserRepository::new(
            db_pool.clone(),
        ));
        let session_repo = Arc::new(
            filehub_database::repositories::session::SessionRepository::new(db_pool.clone()),
        );
        let file_repo = Arc::new(filehub_database::repositories::file::FileRepository::new(
            db_pool.clone(),
        ));
        let folder_repo = Arc::new(
            filehub_database::repositories::folder::FolderRepository::new(db_pool.clone()),
        );
        let storage_repo = Arc::new(
            filehub_database::repositories::storage::StorageRepository::new(db_pool.clone()),
        );
        let permission_repo = Arc::new(
            filehub_database::repositories::permission::AclRepository::new(db_pool.clone()),
        );
        let share_repo = Arc::new(filehub_database::repositories::share::ShareRepository::new(
            db_pool.clone(),
        ));
        let job_repo = Arc::new(filehub_database::repositories::job::JobRepository::new(
            db_pool.clone(),
        ));
        let notification_repo = Arc::new(
            filehub_database::repositories::notification::NotificationRepository::new(
                db_pool.clone(),
            ),
        );
        let audit_repo = Arc::new(
            filehub_database::repositories::audit::AuditLogRepository::new(db_pool.clone()),
        );
        let license_repo = Arc::new(
            filehub_database::repositories::license::LicenseCheckoutRepository::new(
                db_pool.clone(),
            ),
        );
        let snapshot_repo = Arc::new(
            filehub_database::repositories::pool_snapshot::PoolSnapshotRepository::new(
                db_pool.clone(),
            ),
        );

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
        let seat_allocator = Arc::new(filehub_auth::seat::allocator::SeatAllocatorDispatch::new(
            &config.session,
            Arc::clone(&cache),
            Arc::clone(&session_repo),
        ));

        let hook_registry = Arc::new(filehub_plugin::hooks::registry::HookRegistry::new());

        let realtime_engine = Arc::new(
            filehub_realtime::server::RealtimeEngine::new(
                &config.realtime,
                Arc::clone(&jwt_decoder),
                Arc::clone(&session_repo),
                Arc::new(
                    filehub_service::notification::service::NotificationService::new(
                        Arc::clone(&notification_repo),
                        Arc::clone(&cache),
                    ),
                ),
            )
            .await,
        );

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

        let app_state = filehub_api::state::AppState {
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
            hook_registry,
            realtime_engine,
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
        };

        let router = filehub_api::router::build_router(app_state);

        Self {
            router,
            db_pool,
            config,
        }
    }

    /// Clean all test data from the database
    async fn clean_database(pool: &PgPool) {
        let tables = [
            "pool_snapshots",
            "user_session_limits",
            "admin_broadcasts",
            "audit_log",
            "notification_preferences",
            "notifications",
            "license_checkouts",
            "jobs",
            "shares",
            "acl_entries",
            "file_versions",
            "chunked_uploads",
            "files",
            "folders",
            "storages",
            "sessions",
            "users",
        ];

        for table in &tables {
            let query = format!("DELETE FROM {}", table);
            let _ = sqlx::query(&query).execute(pool).await;
        }
    }

    /// Create a test user and return their ID
    pub async fn create_test_user(&self, username: &str, password: &str, role: &str) -> Uuid {
        let hasher = filehub_auth::password::hasher::PasswordHasher::new(&self.config.auth);
        let hash = hasher.hash(password).expect("Failed to hash password");
        let id = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO users (id, username, email, password_hash, display_name, role, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6::user_role, 'active'::user_status, NOW(), NOW())"#,
        )
        .bind(id)
        .bind(username)
        .bind(format!("{}@test.com", username))
        .bind(&hash)
        .bind(username)
        .bind(role)
        .execute(&self.db_pool)
        .await
        .expect("Failed to create test user");

        id
    }

    /// Login and return JWT access token
    pub async fn login(&self, username: &str, password: &str) -> String {
        let body = serde_json::json!({
            "username": username,
            "password": password,
        });

        let response = self
            .request("POST", "/api/auth/login", Some(body), None)
            .await;

        assert_eq!(
            response.status,
            StatusCode::OK,
            "Login failed: {:?}",
            response.body
        );

        response
            .body
            .get("access_token")
            .and_then(|v| v.as_str())
            .expect("No access_token in login response")
            .to_string()
    }

    /// Make an HTTP request to the test app
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> TestResponse {
        let body_str = body
            .map(|b| serde_json::to_string(&b).expect("Failed to serialize body"))
            .unwrap_or_default();

        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json");

        if let Some(token) = token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let req = req
            .body(Body::from(body_str))
            .expect("Failed to build request");

        let response = self
            .router
            .clone()
            .oneshot(req)
            .await
            .expect("Failed to send request");

        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("Failed to read body");

        let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);

        TestResponse { status, body }
    }
}

/// Response from a test request
#[derive(Debug)]
pub struct TestResponse {
    /// HTTP status code
    pub status: StatusCode,
    /// Parsed JSON body
    pub body: Value,
}
