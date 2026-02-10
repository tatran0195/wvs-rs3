//! Local filesystem storage provider.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tracing::debug;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::traits::storage::{ByteStream, StorageObjectMeta, StorageProvider};

/// Local filesystem storage provider.
#[derive(Debug, Clone)]
pub struct LocalStorageProvider {
    /// Root directory for all stored files.
    root: PathBuf,
}

impl LocalStorageProvider {
    /// Create a new local storage provider rooted at the given path.
    pub async fn new(root_path: &str) -> AppResult<Self> {
        let root = PathBuf::from(root_path);
        fs::create_dir_all(&root).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to create storage root: {}", root.display()),
                e,
            )
        })?;
        Ok(Self { root })
    }

    /// Resolve a relative path to an absolute path within the root.
    fn resolve(&self, path: &str) -> PathBuf {
        let clean = path.trim_start_matches('/');
        self.root.join(clean)
    }

    /// Ensure the parent directory of a path exists.
    async fn ensure_parent(&self, path: &Path) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to create parent directory: {}", parent.display()),
                    e,
                )
            })?;
        }
        Ok(())
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    fn provider_type(&self) -> &str {
        "local"
    }

    async fn health_check(&self) -> AppResult<bool> {
        Ok(self.root.exists() && self.root.is_dir())
    }

    async fn read(&self, path: &str) -> AppResult<ByteStream> {
        let full_path = self.resolve(path);
        let file = fs::File::open(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::not_found(format!("File not found: {path}"))
            } else {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to open file: {path}"),
                    e,
                )
            }
        })?;

        let stream = ReaderStream::new(file);
        Ok(Box::pin(stream.map(|r| r.map(|b| b.into()))))
    }

    async fn read_bytes(&self, path: &str) -> AppResult<Bytes> {
        let full_path = self.resolve(path);
        let data = fs::read(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::not_found(format!("File not found: {path}"))
            } else {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to read file: {path}"),
                    e,
                )
            }
        })?;
        Ok(Bytes::from(data))
    }

    async fn write(&self, path: &str, data: Bytes) -> AppResult<()> {
        let full_path = self.resolve(path);
        self.ensure_parent(&full_path).await?;

        fs::write(&full_path, &data).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to write file: {path}"),
                e,
            )
        })?;

        debug!(path, bytes = data.len(), "Wrote file");
        Ok(())
    }

    async fn write_stream(&self, path: &str, mut stream: ByteStream) -> AppResult<u64> {
        let full_path = self.resolve(path);
        self.ensure_parent(&full_path).await?;

        let mut file = fs::File::create(&full_path).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to create file: {path}"),
                e,
            )
        })?;

        let mut total_bytes = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| AppError::with_source(ErrorKind::Storage, "Stream read error", e))?;
            total_bytes += chunk.len() as u64;
            file.write_all(&chunk).await.map_err(|e| {
                AppError::with_source(ErrorKind::Storage, "Failed to write chunk", e)
            })?;
        }

        file.flush()
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Storage, "Failed to flush file", e))?;

        debug!(path, bytes = total_bytes, "Wrote file from stream");
        Ok(total_bytes)
    }

    async fn delete(&self, path: &str) -> AppResult<()> {
        let full_path = self.resolve(path);
        if full_path.exists() {
            fs::remove_file(&full_path).await.map_err(|e| {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to delete file: {path}"),
                    e,
                )
            })?;
        }
        Ok(())
    }

    async fn delete_dir(&self, path: &str) -> AppResult<()> {
        let full_path = self.resolve(path);
        if full_path.exists() {
            fs::remove_dir_all(&full_path).await.map_err(|e| {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to delete directory: {path}"),
                    e,
                )
            })?;
        }
        Ok(())
    }

    async fn copy(&self, from: &str, to: &str) -> AppResult<()> {
        let from_path = self.resolve(from);
        let to_path = self.resolve(to);
        self.ensure_parent(&to_path).await?;

        fs::copy(&from_path, &to_path).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to copy {from} -> {to}"),
                e,
            )
        })?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> AppResult<()> {
        let from_path = self.resolve(from);
        let to_path = self.resolve(to);
        self.ensure_parent(&to_path).await?;

        fs::rename(&from_path, &to_path).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to rename {from} -> {to}"),
                e,
            )
        })?;
        Ok(())
    }

    async fn exists(&self, path: &str) -> AppResult<bool> {
        let full_path = self.resolve(path);
        Ok(full_path.exists())
    }

    async fn metadata(&self, path: &str) -> AppResult<StorageObjectMeta> {
        let full_path = self.resolve(path);
        let meta = fs::metadata(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::not_found(format!("Path not found: {path}"))
            } else {
                AppError::with_source(
                    ErrorKind::Storage,
                    format!("Failed to get metadata: {path}"),
                    e,
                )
            }
        })?;

        let last_modified = meta
            .modified()
            .ok()
            .and_then(|t| chrono::DateTime::<chrono::Utc>::from(t).into());

        let mime_type = if meta.is_file() {
            mime_from_path(path)
        } else {
            None
        };

        Ok(StorageObjectMeta {
            path: path.to_string(),
            size_bytes: meta.len(),
            mime_type,
            last_modified: Some(last_modified.unwrap_or_else(chrono::Utc::now)),
            is_directory: meta.is_dir(),
            checksum_sha256: None,
        })
    }

    async fn list(&self, path: &str) -> AppResult<Vec<StorageObjectMeta>> {
        let full_path = self.resolve(path);
        if !full_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&full_path).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to list directory: {path}"),
                e,
            )
        })?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            AppError::with_source(ErrorKind::Storage, "Failed to read directory entry", e)
        })? {
            let entry_meta = entry.metadata().await.map_err(|e| {
                AppError::with_source(ErrorKind::Storage, "Failed to get entry metadata", e)
            })?;

            let name = entry.file_name().to_string_lossy().to_string();
            let entry_path = if path.is_empty() || path == "/" {
                name.clone()
            } else {
                format!("{}/{}", path.trim_end_matches('/'), name)
            };

            let last_modified = entry_meta
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t));

            entries.push(StorageObjectMeta {
                path: entry_path.clone(),
                size_bytes: entry_meta.len(),
                mime_type: if entry_meta.is_file() {
                    mime_from_path(&entry_path)
                } else {
                    None
                },
                last_modified,
                is_directory: entry_meta.is_dir(),
                checksum_sha256: None,
            });
        }

        entries.sort_by(|a, b| {
            b.is_directory
                .cmp(&a.is_directory)
                .then(a.path.cmp(&b.path))
        });

        Ok(entries)
    }

    async fn create_dir(&self, path: &str) -> AppResult<()> {
        let full_path = self.resolve(path);
        fs::create_dir_all(&full_path).await.map_err(|e| {
            AppError::with_source(
                ErrorKind::Storage,
                format!("Failed to create directory: {path}"),
                e,
            )
        })?;
        Ok(())
    }

    async fn capacity(&self) -> AppResult<(u64, u64)> {
        // Use statvfs on Unix-like systems for filesystem capacity
        // For portability, return 0/0 if we can't determine.
        Ok((0, 0))
    }
}

/// Guess MIME type from a file path extension.
fn mime_from_path(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next()?.to_lowercase();
    let mime = match ext.as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "dwg" => "application/acad",
        "dxf" => "application/dxf",
        "step" | "stp" => "application/step",
        "stl" => "application/sla",
        _ => return None,
    };
    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_read_delete() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalStorageProvider::new(dir.path().to_str().unwrap())
            .await
            .unwrap();

        let data = Bytes::from("hello world");
        provider.write("test/file.txt", data.clone()).await.unwrap();

        assert!(provider.exists("test/file.txt").await.unwrap());

        let read_back = provider.read_bytes("test/file.txt").await.unwrap();
        assert_eq!(read_back, data);

        provider.delete("test/file.txt").await.unwrap();
        assert!(!provider.exists("test/file.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_list() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalStorageProvider::new(dir.path().to_str().unwrap())
            .await
            .unwrap();

        provider
            .write("listdir/a.txt", Bytes::from("a"))
            .await
            .unwrap();
        provider
            .write("listdir/b.txt", Bytes::from("b"))
            .await
            .unwrap();
        provider.create_dir("listdir/subdir").await.unwrap();

        let entries = provider.list("listdir").await.unwrap();
        assert_eq!(entries.len(), 3);
        // Directories come first
        assert!(entries[0].is_directory);
    }

    #[tokio::test]
    async fn test_copy_rename() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalStorageProvider::new(dir.path().to_str().unwrap())
            .await
            .unwrap();

        provider
            .write("orig.txt", Bytes::from("content"))
            .await
            .unwrap();
        provider.copy("orig.txt", "copy.txt").await.unwrap();

        assert!(provider.exists("orig.txt").await.unwrap());
        assert!(provider.exists("copy.txt").await.unwrap());

        provider.rename("copy.txt", "moved.txt").await.unwrap();
        assert!(!provider.exists("copy.txt").await.unwrap());
        assert!(provider.exists("moved.txt").await.unwrap());
    }

    #[test]
    fn test_mime_detection() {
        assert_eq!(mime_from_path("file.pdf"), Some("application/pdf".into()));
        assert_eq!(mime_from_path("img.PNG"), Some("image/png".into()));
        assert_eq!(mime_from_path("noext"), None);
    }
}
