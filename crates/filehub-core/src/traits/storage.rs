//! Storage provider trait for pluggable file storage backends.

use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

use crate::result::AppResult;

/// Metadata about a stored object.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageObjectMeta {
    /// Path within the storage provider.
    pub path: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// MIME type (if known).
    pub mime_type: Option<String>,
    /// Last modified timestamp.
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this is a directory.
    pub is_directory: bool,
    /// SHA-256 checksum (if available).
    pub checksum_sha256: Option<String>,
}

/// A byte stream type used for reading file contents.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

/// Trait for file storage backends.
///
/// Implementations exist for local filesystem, S3,
/// and SMB protocols. The [`StorageProvider`] trait is defined here
/// in `filehub-core` and implemented in `filehub-storage`.
#[async_trait]
pub trait StorageProvider: Send + Sync + std::fmt::Debug + 'static {
    /// Return the provider type name (e.g., "local", "s3").
    fn provider_type(&self) -> &str;

    /// Check whether the provider is healthy and reachable.
    async fn health_check(&self) -> AppResult<bool>;

    /// Read a file and return its byte stream.
    async fn read(&self, path: &str) -> AppResult<ByteStream>;

    /// Read a file into memory as a complete byte vector.
    async fn read_bytes(&self, path: &str) -> AppResult<Bytes>;

    /// Write bytes to a file at the given path.
    async fn write(&self, path: &str, data: Bytes) -> AppResult<()>;

    /// Write a byte stream to a file at the given path.
    async fn write_stream(&self, path: &str, stream: ByteStream) -> AppResult<u64>;

    /// Delete a file at the given path.
    async fn delete(&self, path: &str) -> AppResult<()>;

    /// Delete a directory and all its contents recursively.
    async fn delete_dir(&self, path: &str) -> AppResult<()>;

    /// Copy a file from one path to another within this provider.
    async fn copy(&self, from: &str, to: &str) -> AppResult<()>;

    /// Move (rename) a file from one path to another within this provider.
    async fn rename(&self, from: &str, to: &str) -> AppResult<()>;

    /// Check whether a file or directory exists at the given path.
    async fn exists(&self, path: &str) -> AppResult<bool>;

    /// Get metadata about a file or directory.
    async fn metadata(&self, path: &str) -> AppResult<StorageObjectMeta>;

    /// List the contents of a directory.
    async fn list(&self, path: &str) -> AppResult<Vec<StorageObjectMeta>>;

    /// Create a directory (and any missing parents).
    async fn create_dir(&self, path: &str) -> AppResult<()>;

    /// Get the total and used capacity of this storage backend.
    async fn capacity(&self) -> AppResult<(u64, u64)>;
}
