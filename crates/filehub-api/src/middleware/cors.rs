//! CORS layer configuration.

use axum::http::{HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};

use filehub_core::config::CorsConfig;

/// Builds a CORS tower layer from configuration.
pub fn build_cors_layer(config: &CorsConfig) -> CorsLayer {
    let mut layer = CorsLayer::new();

    // Origins
    if config.allowed_origins.contains(&"*".to_string()) {
        layer = layer.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer = layer.allow_origin(origins);
    }

    // Methods
    let methods: Vec<Method> = config
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    layer = layer.allow_methods(methods);

    // Headers
    if config.allowed_headers.contains(&"*".to_string()) {
        layer = layer.allow_headers(Any);
    }

    layer = layer.max_age(std::time::Duration::from_secs(
        config.max_age_seconds as u64,
    ));

    layer
}
