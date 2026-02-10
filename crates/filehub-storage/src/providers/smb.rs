//! SMB/CIFS network share storage provider (stub — requires `smb` feature).

use async_trait::async_trait;
use bytes::Bytes;

use filehub_core::error::AppError;
use filehub_core::result::AppResult;
use filehub_core::traits::storage::{ByteStream, StorageObjectMeta, StorageProvider};

/// SMB storage provider.
#[derive(Debug, Clone)]
pub struct SmbStorageProvider {
    share_path: String,
}

impl SmbStorageProvider {
    /// Create a new SMB storage provider.
    pub fn new(share_path: &str, _username: &str, _password: &str, _domain: &str) -> Self {
        Self {
            share_path: share_path.to_string(),
        }
    }
}

#[async_trait]
impl StorageProvider for SmbStorageProvider {
    fn provider_type(&self) -> &str {
        "smb"
    }
    async fn health_check(&self) -> AppResult<bool> {
        Err(AppError::not_implemented(
            "SMB health check not yet implemented",
        ))
    }
    async fn read(&self, _p: &str) -> AppResult<ByteStream> {
        Err(AppError::not_implemented("SMB read not yet implemented"))
    }
    async fn read_bytes(&self, _p: &str) -> AppResult<Bytes> {
        Err(AppError::not_implemented(
            "SMB read_bytes not yet implemented",
        ))
    }
    async fn write(&self, _p: &str, _d: Bytes) -> AppResult<()> {
        Err(AppError::not_implemented("SMB write not yet implemented"))
    }
    async fn write_stream(&self, _p: &str, _s: ByteStream) -> AppResult<u64> {
        Err(AppError::not_implemented(
            "SMB write_stream not yet implemented",
        ))
    }
    async fn delete(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented("SMB delete not yet implemented"))
    }
    async fn delete_dir(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "SMB delete_dir not yet implemented",
        ))
    }
    async fn copy(&self, _f: &str, _t: &str) -> AppResult<()> {
        Err(AppError::not_implemented("SMB copy not yet implemented"))
    }
    async fn rename(&self, _f: &str, _t: &str) -> AppResult<()> {
        Err(AppError::not_implemented("SMB rename not yet implemented"))
    }
    async fn exists(&self, _p: &str) -> AppResult<bool> {
        Err(AppError::not_implemented("SMB exists not yet implemented"))
    }
    async fn metadata(&self, _p: &str) -> AppResult<StorageObjectMeta> {
        Err(AppError::not_implemented(
            "SMB metadata not yet implemented",
        ))
    }
    async fn list(&self, _p: &str) -> AppResult<Vec<StorageObjectMeta>> {
        Err(AppError::not_implemented("SMB list not yet implemented"))
    }
    async fn create_dir(&self, _p: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "SMB create_dir not yet implemented",
        ))
    }
    async fn capacity(&self) -> AppResult<(u64, u64)> {
        Ok((0, 0))
    }
}
