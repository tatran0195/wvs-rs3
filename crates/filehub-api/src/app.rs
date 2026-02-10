//! Application builder — wires router + middleware + state into an Axum app.

use axum::Router;
use tower_http::trace::TraceLayer;

use filehub_core::config::CorsConfig;

use crate::middleware::compression::build_compression_layer;
use crate::middleware::cors::build_cors_layer;
use crate::router::build_router;
use crate::state::AppState;

/// Builds the complete Axum application with all routes and middleware.
pub fn build_app(state: AppState, cors_config: &CorsConfig) -> Router {
    build_router()
        .layer(build_compression_layer())
        .layer(build_cors_layer(cors_config))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
