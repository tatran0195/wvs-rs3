//! S3-compatible object storage provider (stub — requires `s3` feature).

use async_trait::async_trait;
use bytes::Bytes;

use filehub_core::error::AppError;
use filehub_core::result::AppResult;
use filehub_core::traits::storage::{ByteStream, StorageObjectMeta, StorageProvider};

/// S3-compatible storage provider.
#[derive(Debug, Clone)]
pub struct S3StorageProvider {
    bucket: String,
    region: String,
}

impl S3StorageProvider {
    /// Create a new S3 storage provider.
    pub async fn new(
        endpoint: &str,
        region: &str,
        bucket: &str,
        _access_key: &str,
        _secret_key: &str,
    ) -> AppResult<Self> {
        tracing::info!(endpoint, region, bucket, "Initializing S3 storage provider");
        Ok(Self {
            bucket: bucket.to_string(),
            region: region.to_string(),
        })
    }
}

#[async_trait]
impl StorageProvider for S3StorageProvider {
    fn provider_type(&self) -> &str {
        "s3"
    }

    async fn health_check(&self) -> AppResult<bool> {
        Err(AppError::not_implemented(
            "S3 health check not yet implemented",
        ))
    }

    async fn read(&self, _path: &str) -> AppResult<ByteStream> {
        Err(AppError::not_implemented("S3 read not yet implemented"))
    }

    async fn read_bytes(&self, _path: &str) -> AppResult<Bytes> {
        Err(AppError::not_implemented(
            "S3 read_bytes not yet implemented",
        ))
    }

    async fn write(&self, _path: &str, _data: Bytes) -> AppResult<()> {
        Err(AppError::not_implemented("S3 write not yet implemented"))
    }

    async fn write_stream(&self, _path: &str, _stream: ByteStream) -> AppResult<u64> {
        Err(AppError::not_implemented(
            "S3 write_stream not yet implemented",
        ))
    }

    async fn delete(&self, _path: &str) -> AppResult<()> {
        Err(AppError::not_implemented("S3 delete not yet implemented"))
    }

    async fn delete_dir(&self, _path: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "S3 delete_dir not yet implemented",
        ))
    }

    async fn copy(&self, _from: &str, _to: &str) -> AppResult<()> {
        Err(AppError::not_implemented("S3 copy not yet implemented"))
    }

    async fn rename(&self, _from: &str, _to: &str) -> AppResult<()> {
        Err(AppError::not_implemented("S3 rename not yet implemented"))
    }

    async fn exists(&self, _path: &str) -> AppResult<bool> {
        Err(AppError::not_implemented("S3 exists not yet implemented"))
    }

    async fn metadata(&self, _path: &str) -> AppResult<StorageObjectMeta> {
        Err(AppError::not_implemented("S3 metadata not yet implemented"))
    }

    async fn list(&self, _path: &str) -> AppResult<Vec<StorageObjectMeta>> {
        Err(AppError::not_implemented("S3 list not yet implemented"))
    }

    async fn create_dir(&self, _path: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "S3 create_dir not yet implemented",
        ))
    }

    async fn capacity(&self) -> AppResult<(u64, u64)> {
        Ok((0, 0))
    }
}
