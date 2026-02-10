//! WebDAV client storage provider (stub — requires `webdav-client` feature).

use async_trait::async_trait;
use bytes::Bytes;

use filehub_core::error::AppError;
use filehub_core::result::AppResult;
use filehub_core::traits::storage::{ByteStream, StorageObjectMeta, StorageProvider};

/// WebDAV client storage provider.
#[derive(Debug, Clone)]
pub struct WebDavStorageProvider {
    base_url: String,
}

impl WebDavStorageProvider {
    /// Create a new WebDAV storage provider.
    pub fn new(base_url: &str, _username: &str, _password: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl StorageProvider for WebDavStorageProvider {
    fn provider_type(&self) -> &str {
        "webdav"
    }
    async fn health_check(&self) -> AppResult<bool> {
        Err(AppError::not_implemented(
            "WebDAV health check not yet implemented",
        ))
    }
    async fn read(&self, _p: &str) -> AppResult<ByteStream> {
        Err(AppError::not_implemented("WebDAV read not yet implemented"))
    }
    async fn read_bytes(&self, _p: &str) -> AppResult<Bytes> {
        Err(AppError::not_implemented(
            "WebDAV read_bytes not yet implemented",
        ))
    }
    async fn write(&self, _p: &str, _d: Bytes) -> AppResult<()> {
        Err(AppError::not_implemented(
            "WebDAV write not yet implemented",
        ))
    }
    async fn write_stream(&self, _p: &str, _s: ByteStream) -> AppResult<u64> {
        Err(AppError::not_implemented(
            "WebDAV write_stream not yet implemented",
        ))
    }
    async fn delete(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "WebDAV delete not yet implemented",
        ))
    }
    async fn delete_dir(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "WebDAV delete_dir not yet implemented",
        ))
    }
    async fn copy(&self, _f: &str, _t: &str) -> AppResult<()> {
        Err(AppError::not_implemented("WebDAV copy not yet implemented"))
    }
    async fn rename(&self, _f: &str, _t: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "WebDAV rename not yet implemented",
        ))
    }
    async fn exists(&self, _p: &str) -> AppResult<bool> {
        Err(AppError::not_implemented(
            "WebDAV exists not yet implemented",
        ))
    }
    async fn metadata(&self, _p: &str) -> AppResult<StorageObjectMeta> {
        Err(AppError::not_implemented(
            "WebDAV metadata not yet implemented",
        ))
    }
    async fn list(&self, _p: &str) -> AppResult<Vec<StorageObjectMeta>> {
        Err(AppError::not_implemented("WebDAV list not yet implemented"))
    }
    async fn create_dir(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "WebDAV create_dir not yet implemented",
        ))
    }
    async fn capacity(&self) -> AppResult<(u64, u64)> {
        Ok((0, 0))
    }
}
