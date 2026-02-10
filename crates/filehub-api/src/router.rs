//! Route definitions for the FileHub HTTP API.
//!
//! All routes are organized by domain and mounted under `/api`.
//! The router receives `AppState` and passes it to all handlers via Axum's `State` extractor.

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware as axum_middleware,
    routing::{delete, get, post, put},
};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::middleware;
use crate::state::AppState;

/// Build the complete Axum router with all routes and middleware.
///
/// Receives the fully-constructed `AppState` and threads it through
/// every route via `.with_state(state)`.
pub fn build_router(state: AppState) -> Router {
    let max_upload = state.config.storage.max_upload_size_bytes as usize;

    let api_routes = Router::new()
        .merge(auth_routes())
        .merge(user_routes())
        .merge(file_routes())
        .merge(folder_routes())
        .merge(share_routes())
        .merge(permission_routes())
        .merge(storage_routes())
        .merge(notification_routes())
        .merge(presence_routes())
        .merge(search_routes())
        .merge(admin_routes())
        .merge(health_routes());

    let ws_routes = Router::new().route("/ws", get(handlers::ws::ws_upgrade));

    let cors = build_cors_layer(&state);

    Router::new()
        .nest("/api", api_routes)
        .merge(ws_routes)
        .layer(DefaultBodyLimit::max(max_upload))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::logging::request_logging,
        ))
        .with_state(state)
}

/// Auth endpoints: login, logout, refresh, me
fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/me", get(handlers::auth::me))
}

/// User self-service endpoints
fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/users/me", get(handlers::user::get_profile))
        .route("/users/me", put(handlers::user::update_profile))
        .route("/users/me/password", put(handlers::user::change_password))
}

/// File CRUD, upload, download, versions
fn file_routes() -> Router<AppState> {
    Router::new()
        .route("/files", get(handlers::file::list_files))
        .route("/files/:id", get(handlers::file::get_file))
        .route("/files/:id", put(handlers::file::update_file))
        .route("/files/:id", delete(handlers::file::delete_file))
        .route("/files/:id/download", get(handlers::file::download_file))
        .route("/files/:id/preview", get(handlers::file::preview_file))
        .route("/files/:id/versions", get(handlers::file::list_versions))
        .route(
            "/files/:id/versions/:ver",
            get(handlers::file::download_version),
        )
        .route("/files/upload", post(handlers::file::upload_file))
        .route(
            "/files/upload/initiate",
            post(handlers::file::initiate_chunked_upload),
        )
        .route(
            "/files/upload/:id/chunk/:n",
            put(handlers::file::upload_chunk),
        )
        .route(
            "/files/upload/:id/complete",
            post(handlers::file::complete_chunked_upload),
        )
        .route("/files/:id/move", put(handlers::file::move_file))
        .route("/files/:id/copy", post(handlers::file::copy_file))
        .route("/files/:id/lock", post(handlers::file::lock_file))
        .route("/files/:id/unlock", post(handlers::file::unlock_file))
}

/// Folder CRUD and tree
fn folder_routes() -> Router<AppState> {
    Router::new()
        .route("/folders", get(handlers::folder::list_root_folders))
        .route("/folders", post(handlers::folder::create_folder))
        .route("/folders/:id", get(handlers::folder::get_folder))
        .route("/folders/:id", put(handlers::folder::update_folder))
        .route("/folders/:id", delete(handlers::folder::delete_folder))
        .route(
            "/folders/:id/children",
            get(handlers::folder::list_children),
        )
        .route("/folders/:id/tree", get(handlers::folder::get_tree))
        .route("/folders/:id/move", put(handlers::folder::move_folder))
}

/// Share CRUD and public access
fn share_routes() -> Router<AppState> {
    Router::new()
        .route("/shares", get(handlers::share::list_shares))
        .route("/shares", post(handlers::share::create_share))
        .route("/shares/:id", get(handlers::share::get_share))
        .route("/shares/:id", put(handlers::share::update_share))
        .route("/shares/:id", delete(handlers::share::delete_share))
        .route("/s/:token", get(handlers::share::access_shared))
        .route("/s/:token/verify", post(handlers::share::verify_share))
}

/// Permission/ACL management
fn permission_routes() -> Router<AppState> {
    Router::new()
        .route("/permissions/:type/:id", get(handlers::permission::get_acl))
        .route(
            "/permissions/:type/:id",
            post(handlers::permission::add_acl),
        )
        .route(
            "/permissions/entry/:id",
            put(handlers::permission::update_acl),
        )
        .route(
            "/permissions/entry/:id",
            delete(handlers::permission::delete_acl),
        )
}

/// Storage listing and usage
fn storage_routes() -> Router<AppState> {
    Router::new()
        .route("/storages", get(handlers::storage::list_storages))
        .route("/storages/:id", get(handlers::storage::get_storage))
        .route("/storages/:id/usage", get(handlers::storage::get_usage))
        .route(
            "/storages/:id/transfer",
            post(handlers::storage::initiate_transfer),
        )
}

/// Notification endpoints
fn notification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/notifications",
            get(handlers::notification::list_notifications),
        )
        .route(
            "/notifications/unread-count",
            get(handlers::notification::unread_count),
        )
        .route(
            "/notifications/:id/read",
            put(handlers::notification::mark_read),
        )
        .route(
            "/notifications/read-all",
            put(handlers::notification::mark_all_read),
        )
        .route(
            "/notifications/:id",
            delete(handlers::notification::dismiss),
        )
        .route(
            "/notifications/preferences",
            get(handlers::notification::get_preferences),
        )
        .route(
            "/notifications/preferences",
            put(handlers::notification::update_preferences),
        )
}

/// Presence endpoints
fn presence_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/presence/online",
            get(handlers::notification::online_users),
        )
        .route(
            "/presence/status",
            put(handlers::notification::update_presence),
        )
}

/// Search endpoints
fn search_routes() -> Router<AppState> {
    Router::new().route("/files/search", get(handlers::search::search_files))
}

/// Admin-only endpoints
fn admin_routes() -> Router<AppState> {
    Router::new()
        // User management
        .route("/admin/users", get(handlers::admin::users::list_users))
        .route("/admin/users", post(handlers::admin::users::create_user))
        .route("/admin/users/:id", get(handlers::admin::users::get_user))
        .route("/admin/users/:id", put(handlers::admin::users::update_user))
        .route(
            "/admin/users/:id/role",
            put(handlers::admin::users::change_role),
        )
        .route(
            "/admin/users/:id/status",
            put(handlers::admin::users::change_status),
        )
        .route(
            "/admin/users/:id/reset-password",
            put(handlers::admin::users::reset_password),
        )
        .route(
            "/admin/users/:id",
            delete(handlers::admin::users::delete_user),
        )
        // Storage management
        .route(
            "/admin/storages",
            get(handlers::admin::storages::list_storages),
        )
        .route(
            "/admin/storages",
            post(handlers::admin::storages::add_storage),
        )
        .route(
            "/admin/storages/:id",
            put(handlers::admin::storages::update_storage),
        )
        .route(
            "/admin/storages/:id",
            delete(handlers::admin::storages::remove_storage),
        )
        .route(
            "/admin/storages/:id/test",
            post(handlers::admin::storages::test_storage),
        )
        .route(
            "/admin/storages/:id/sync",
            post(handlers::admin::storages::sync_storage),
        )
        // Session management
        .route(
            "/admin/sessions",
            get(handlers::admin::sessions::list_sessions),
        )
        .route(
            "/admin/sessions/:id",
            get(handlers::admin::sessions::get_session),
        )
        .route(
            "/admin/sessions/:id/terminate",
            post(handlers::admin::sessions::terminate_session),
        )
        .route(
            "/admin/sessions/terminate-bulk",
            post(handlers::admin::sessions::terminate_bulk),
        )
        .route(
            "/admin/sessions/terminate-all",
            post(handlers::admin::sessions::terminate_all),
        )
        .route(
            "/admin/sessions/:id/send-message",
            post(handlers::admin::sessions::send_message),
        )
        // Session limits
        .route(
            "/admin/session-limits",
            get(handlers::admin::sessions::get_limits),
        )
        .route(
            "/admin/session-limits/role/:role",
            put(handlers::admin::sessions::update_role_limit),
        )
        .route(
            "/admin/session-limits/strategy",
            put(handlers::admin::sessions::update_strategy),
        )
        .route(
            "/admin/session-limits/user/:id",
            post(handlers::admin::sessions::set_user_limit),
        )
        .route(
            "/admin/session-limits/user/:id",
            delete(handlers::admin::sessions::remove_user_limit),
        )
        // Broadcast
        .route(
            "/admin/broadcast",
            post(handlers::admin::broadcast::send_broadcast),
        )
        .route(
            "/admin/broadcast/history",
            get(handlers::admin::broadcast::broadcast_history),
        )
        // License
        .route(
            "/admin/license/pool",
            get(handlers::admin::license::pool_status),
        )
        .route(
            "/admin/license/pool/history",
            get(handlers::admin::license::pool_history),
        )
        .route(
            "/admin/license/pool/reconcile",
            post(handlers::admin::license::pool_reconcile),
        )
        // Jobs
        .route("/admin/jobs", get(handlers::admin::jobs::list_jobs))
        .route("/admin/jobs/:id", get(handlers::admin::jobs::get_job))
        .route(
            "/admin/jobs/:id/cancel",
            post(handlers::admin::jobs::cancel_job),
        )
        .route(
            "/admin/jobs/:id/retry",
            post(handlers::admin::jobs::retry_job),
        )
        // Reports
        .route(
            "/admin/reports/weekly",
            get(handlers::admin::reports::weekly_report),
        )
        .route(
            "/admin/reports/storage-usage",
            get(handlers::admin::reports::storage_usage),
        )
        // Audit
        .route("/admin/audit", get(handlers::admin::audit::search_audit))
        .route(
            "/admin/audit/export",
            get(handlers::admin::audit::export_audit),
        )
}

/// Health check endpoints (no auth required)
fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/health/detailed", get(handlers::health::detailed_health))
}

/// Build CORS layer from configuration
fn build_cors_layer(state: &AppState) -> CorsLayer {
    use http::Method;
    use tower_http::cors::{AllowOrigin, Any};

    let cors_config = &state.config.server.cors;

    let mut cors = CorsLayer::new();

    if cors_config.allowed_origins.contains(&"*".to_string()) {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<http::HeaderValue> = cors_config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }

    let methods: Vec<Method> = cors_config
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);

    if cors_config.allowed_headers.contains(&"*".to_string()) {
        cors = cors.allow_headers(Any);
    }

    cors = cors.max_age(std::time::Duration::from_secs(cors_config.max_age_seconds));

    cors
}
