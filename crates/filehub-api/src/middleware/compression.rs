//! Response compression layer.

use tower_http::compression::CompressionLayer;

/// Builds a compression layer (gzip).
pub fn build_compression_layer() -> CompressionLayer {
    CompressionLayer::new()
}
