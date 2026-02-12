//! Storage provider configuration.

use serde::{Deserialize, Serialize};

/// Top-level storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Root directory for all runtime data.
    #[serde(default = "default_data_root")]
    pub data_root: String,
    /// Default storage provider to use.
    #[serde(default = "default_provider")]
    pub default_provider: String,
    /// Maximum upload size in bytes (default 5 GB).
    #[serde(default = "default_max_upload")]
    pub max_upload_size_bytes: u64,
    /// Chunk size in bytes for chunked uploads (default 5 MB).
    #[serde(default = "default_chunk_size")]
    pub chunk_size_bytes: u64,
    /// Thumbnail generation sizes.
    #[serde(default = "default_thumbnail_sizes")]
    pub thumbnail_sizes: Vec<u32>,
    /// Local filesystem storage configuration.
    #[serde(default)]
    pub local: LocalStorageConfig,
    /// S3-compatible storage configuration.
    #[serde(default)]
    pub s3: S3StorageConfig,
    /// Configuration for file conversions (e.g. CAD).
    #[serde(default)]
    pub conversions: ConversionConfig,
}

/// Configuration for file conversions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConversionConfig {
    /// Whether conversion is enabled.
    pub enabled: bool,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// Local filesystem storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    /// Root path for local file storage.
    #[serde(default = "default_local_root")]
    pub root_path: String,
}

impl Default for LocalStorageConfig {
    fn default() -> Self {
        Self {
            root_path: default_local_root(),
        }
    }
}

/// S3-compatible object storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct S3StorageConfig {
    /// Whether S3 storage is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// S3 endpoint URL (for non-AWS services like MinIO).
    #[serde(default)]
    pub endpoint: String,
    /// AWS region.
    #[serde(default = "default_region")]
    pub region: String,
    /// S3 bucket name.
    #[serde(default)]
    pub bucket: String,
    /// Access key ID.
    #[serde(default)]
    pub access_key: String,
    /// Secret access key.
    #[serde(default)]
    pub secret_key: String,
}

fn default_data_root() -> String {
    "./data".to_string()
}

fn default_provider() -> String {
    "local".to_string()
}

fn default_max_upload() -> u64 {
    5_368_709_120 // 5 GB
}

fn default_chunk_size() -> u64 {
    5_242_880 // 5 MB
}

fn default_thumbnail_sizes() -> Vec<u32> {
    vec![64, 128, 256, 512]
}

fn default_local_root() -> String {
    "./data/storage/local".to_string()
}

fn default_region() -> String {
    "us-east-1".to_string()
}
