//! File repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::file::chunk::ChunkedUpload;
use filehub_entity::file::model::{CreateFile, File};
use filehub_entity::file::version::FileVersion;

/// Repository for file CRUD and query operations.
#[derive(Debug, Clone)]
pub struct FileRepository {
    pool: PgPool,
}

impl FileRepository {
    /// Create a new file repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a file by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<File>> {
        sqlx::query_as::<_, File>("SELECT * FROM files WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find file", e))
    }

    /// List files in a folder with pagination.
    pub async fn find_by_folder(
        &self,
        folder_id: Uuid,
        page: &PageRequest,
    ) -> AppResult<PageResponse<File>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE folder_id = $1")
            .bind(folder_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count files", e))?;

        let files = sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE folder_id = $1 ORDER BY name ASC LIMIT $2 OFFSET $3",
        )
        .bind(folder_id)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list files", e))?;

        Ok(PageResponse::new(
            files,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Find a file by folder ID and name (for duplicate checking).
    pub async fn find_by_folder_and_name(
        &self,
        folder_id: Uuid,
        name: &str,
    ) -> AppResult<Option<File>> {
        sqlx::query_as::<_, File>("SELECT * FROM files WHERE folder_id = $1 AND name = $2")
            .bind(folder_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find file by name", e)
            })
    }

    /// Update a file record.
    pub async fn update(&self, file: &File) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET folder_id = $2, storage_id = $3, name = $4, storage_path = $5, \
             mime_type = $6, size_bytes = $7, checksum_sha256 = $8, metadata = $9, \
             current_version = $10, is_locked = $11, locked_by = $12, locked_at = $13, \
             owner_id = $14, updated_at = $15 \
             WHERE id = $1 RETURNING *",
        )
        .bind(file.id)
        .bind(file.folder_id)
        .bind(file.storage_id)
        .bind(&file.name)
        .bind(&file.storage_path)
        .bind(&file.mime_type)
        .bind(file.size_bytes)
        .bind(&file.checksum_sha256)
        .bind(&file.metadata)
        .bind(file.current_version)
        .bind(file.is_locked)
        .bind(file.locked_by)
        .bind(file.locked_at)
        .bind(file.owner_id)
        .bind(file.updated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update file", e))?
        .ok_or_else(|| AppError::not_found(format!("File {} not found", file.id)))
    }

    /// Increment the current version of a file.
    pub async fn increment_version(&self, file_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE files SET current_version = current_version + 1, updated_at = NOW() WHERE id = $1"
        )
        .bind(file_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to increment version", e))?;
        Ok(())
    }

    /// Full-text search across file names and metadata descriptions.
    pub async fn search(
        &self,
        query: &str,
        storage_id: Option<Uuid>,
        page: &PageRequest,
    ) -> AppResult<PageResponse<File>> {
        let ts_query = query.split_whitespace().collect::<Vec<_>>().join(" & ");

        if let Some(sid) = storage_id {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM files \
                 WHERE storage_id = $1 AND to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')) @@ to_tsquery('english', $2)"
            )
                .bind(sid)
                .bind(&ts_query)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count search results", e))?;

            let files = sqlx::query_as::<_, File>(
                "SELECT * FROM files \
                 WHERE storage_id = $1 AND to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')) @@ to_tsquery('english', $2) \
                 ORDER BY ts_rank(to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')), to_tsquery('english', $2)) DESC \
                 LIMIT $3 OFFSET $4"
            )
                .bind(sid)
                .bind(&ts_query)
                .bind(page.limit() as i64)
                .bind(page.offset() as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to search files", e))?;

            Ok(PageResponse::new(
                files,
                page.page,
                page.page_size,
                total as u64,
            ))
        } else {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM files \
                 WHERE to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')) @@ to_tsquery('english', $1)"
            )
                .bind(&ts_query)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count search results", e))?;

            let files = sqlx::query_as::<_, File>(
                "SELECT * FROM files \
                 WHERE to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')) @@ to_tsquery('english', $1) \
                 ORDER BY ts_rank(to_tsvector('english', name || ' ' || COALESCE(metadata->>'description', '')), to_tsquery('english', $1)) DESC \
                 LIMIT $2 OFFSET $3"
            )
                .bind(&ts_query)
                .bind(page.limit() as i64)
                .bind(page.offset() as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to search files", e))?;

            Ok(PageResponse::new(
                files,
                page.page,
                page.page_size,
                total as u64,
            ))
        }
    }

    /// Create a new file record.
    pub async fn create(&self, data: &CreateFile) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "INSERT INTO files (folder_id, storage_id, name, storage_path, mime_type, size_bytes, checksum_sha256, metadata, owner_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
        )
            .bind(data.folder_id)
            .bind(data.storage_id)
            .bind(&data.name)
            .bind(&data.storage_path)
            .bind(&data.mime_type)
            .bind(data.size_bytes)
            .bind(&data.checksum_sha256)
            .bind(&data.metadata)
            .bind(data.owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("files_folder_id_name_key") => {
                    AppError::conflict(format!("File '{}' already exists in this folder", data.name))
                }
                _ => AppError::with_source(ErrorKind::Database, "Failed to create file", e),
            })
    }

    /// Update file metadata.
    pub async fn update_metadata(
        &self,
        file_id: Uuid,
        metadata: &serde_json::Value,
    ) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET metadata = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(file_id)
        .bind(metadata)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update metadata", e))?
        .ok_or_else(|| AppError::not_found(format!("File {file_id} not found")))
    }

    /// Move file to a different folder.
    pub async fn move_file(
        &self,
        file_id: Uuid,
        new_folder_id: Uuid,
        new_storage_path: &str,
    ) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET folder_id = $2, storage_path = $3, updated_at = NOW() WHERE id = $1 RETURNING *"
        )
            .bind(file_id)
            .bind(new_folder_id)
            .bind(new_storage_path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to move file", e))?
            .ok_or_else(|| AppError::not_found(format!("File {file_id} not found")))
    }

    /// Rename a file.
    pub async fn rename(&self, file_id: Uuid, new_name: &str) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET name = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(file_id)
        .bind(new_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to rename file", e))?
        .ok_or_else(|| AppError::not_found(format!("File {file_id} not found")))
    }

    /// Lock a file.
    pub async fn lock_file(&self, file_id: Uuid, locked_by: Uuid) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET is_locked = TRUE, locked_by = $2, locked_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND (is_locked = FALSE OR is_locked IS NULL) RETURNING *"
        )
            .bind(file_id)
            .bind(locked_by)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to lock file", e))?
            .ok_or_else(|| AppError::conflict(format!("File {file_id} is already locked or not found")))
    }

    /// Unlock a file.
    pub async fn unlock_file(&self, file_id: Uuid) -> AppResult<File> {
        sqlx::query_as::<_, File>(
            "UPDATE files SET is_locked = FALSE, locked_by = NULL, locked_at = NULL, updated_at = NOW() \
             WHERE id = $1 RETURNING *"
        )
            .bind(file_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to unlock file", e))?
            .ok_or_else(|| AppError::not_found(format!("File {file_id} not found")))
    }

    /// Delete a file.
    pub async fn delete(&self, file_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM files WHERE id = $1")
            .bind(file_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to delete file", e))?;
        Ok(result.rows_affected() > 0)
    }

    // -- File Versions --

    /// List all versions of a file.
    pub async fn find_versions(&self, file_id: Uuid) -> AppResult<Vec<FileVersion>> {
        sqlx::query_as::<_, FileVersion>(
            "SELECT * FROM file_versions WHERE file_id = $1 ORDER BY version_number DESC",
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list file versions", e))
    }

    /// Find a specific version of a file.
    pub async fn find_version(
        &self,
        file_id: Uuid,
        version_number: i32,
    ) -> AppResult<Option<FileVersion>> {
        sqlx::query_as::<_, FileVersion>(
            "SELECT * FROM file_versions WHERE file_id = $1 AND version_number = $2",
        )
        .bind(file_id)
        .bind(version_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find file version", e))
    }

    /// Create a new file version.
    pub async fn create_version(
        &self,
        file_id: Uuid,
        version_number: i32,
        storage_path: &str,
        size_bytes: i64,
        checksum_sha256: Option<&str>,
        created_by: Uuid,
        comment: Option<&str>,
    ) -> AppResult<FileVersion> {
        sqlx::query_as::<_, FileVersion>(
            "INSERT INTO file_versions (file_id, version_number, storage_path, size_bytes, checksum_sha256, created_by, comment) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
        )
            .bind(file_id)
            .bind(version_number)
            .bind(storage_path)
            .bind(size_bytes)
            .bind(checksum_sha256)
            .bind(created_by)
            .bind(comment)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create file version", e))
    }

    /// Delete old file versions exceeding the retention limit per file.
    pub async fn delete_old_versions(&self, max_versions: i64) -> AppResult<u64> {
        let result = sqlx::query(
            "DELETE FROM file_versions WHERE id IN (\
                SELECT id FROM (\
                    SELECT id, ROW_NUMBER() OVER (PARTITION BY file_id ORDER BY version_number DESC) as r_num \
                    FROM file_versions\
                ) t WHERE t.r_num > $1\
             )",
        )
        .bind(max_versions)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to cleanup old versions", e))?;
        Ok(result.rows_affected())
    }

    // -- Chunked Uploads --

    /// Find a chunked upload by ID.
    pub async fn find_chunked_upload(&self, id: Uuid) -> AppResult<Option<ChunkedUpload>> {
        sqlx::query_as::<_, ChunkedUpload>("SELECT * FROM chunked_uploads WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find chunked upload", e)
            })
    }

    /// Create a chunked upload session.
    pub async fn create_chunked_upload(
        &self,
        user_id: Uuid,
        storage_id: Uuid,
        target_folder_id: Uuid,
        file_name: &str,
        file_size: i64,
        mime_type: Option<&str>,
        chunk_size: i32,
        total_chunks: i32,
        checksum_sha256: Option<&str>,
        temp_path: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<ChunkedUpload> {
        sqlx::query_as::<_, ChunkedUpload>(
            "INSERT INTO chunked_uploads \
             (user_id, storage_id, target_folder_id, file_name, file_size, mime_type, chunk_size, total_chunks, checksum_sha256, temp_path, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *"
        )
            .bind(user_id)
            .bind(storage_id)
            .bind(target_folder_id)
            .bind(file_name)
            .bind(file_size)
            .bind(mime_type)
            .bind(chunk_size)
            .bind(total_chunks)
            .bind(checksum_sha256)
            .bind(temp_path)
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create chunked upload", e))
    }

    /// Update uploaded chunks list.
    pub async fn update_chunked_upload_chunks(
        &self,
        upload_id: Uuid,
        uploaded_chunks: &serde_json::Value,
    ) -> AppResult<()> {
        sqlx::query("UPDATE chunked_uploads SET uploaded_chunks = $2 WHERE id = $1")
            .bind(upload_id)
            .bind(uploaded_chunks)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update chunks", e)
            })?;
        Ok(())
    }

    /// Update chunked upload status.
    pub async fn update_chunked_upload_status(
        &self,
        upload_id: Uuid,
        status: &str,
    ) -> AppResult<()> {
        sqlx::query("UPDATE chunked_uploads SET status = $2 WHERE id = $1")
            .bind(upload_id)
            .bind(status)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update upload status", e)
            })?;
        Ok(())
    }

    /// Add a chunk to the uploaded_chunks list.
    pub async fn add_uploaded_chunk(&self, upload_id: Uuid, chunk_number: i32) -> AppResult<()> {
        sqlx::query(
            "UPDATE chunked_uploads SET uploaded_chunks = uploaded_chunks || $2::jsonb \
             WHERE id = $1",
        )
        .bind(upload_id)
        .bind(serde_json::json!([chunk_number]))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to add chunk", e))?;
        Ok(())
    }

    /// Complete a chunked upload.
    pub async fn complete_chunked_upload(&self, upload_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE chunked_uploads SET status = 'completed', completed_at = NOW() WHERE id = $1",
        )
        .bind(upload_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to complete upload", e))?;
        Ok(())
    }

    /// Clean up expired chunked uploads.
    pub async fn cleanup_expired_uploads(&self) -> AppResult<u64> {
        let result = sqlx::query(
            "DELETE FROM chunked_uploads WHERE status = 'uploading' AND expires_at < NOW()",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to cleanup uploads", e))?;
        Ok(result.rows_affected())
    }

    /// Find all expired chunked uploads (status = uploading AND expires_at < NOW())
    pub async fn find_expired_uploads(&self) -> AppResult<Vec<ChunkedUpload>> {
        sqlx::query_as::<_, ChunkedUpload>(
            "SELECT * FROM chunked_uploads WHERE status = 'uploading' AND expires_at < NOW()",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find expired uploads", e)
        })
    }

    /// Delete a chunked upload record by ID.
    pub async fn delete_upload(&self, upload_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM chunked_uploads WHERE id = $1")
            .bind(upload_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to delete upload", e)
            })?;
        Ok(())
    }

    /// Count total files.
    pub async fn count_all(&self) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count files", e))?;
        Ok(count)
    }

    /// Count files created since a specific time.
    pub async fn count_created_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE created_at >= $1")
            .bind(since)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count new files", e)
            })?;
        Ok(count)
    }

    /// Total size of all files in bytes.
    pub async fn total_size_bytes(&self) -> AppResult<i64> {
        let size: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size_bytes), 0) FROM files")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to calculate storage size", e)
            })?;
        Ok(size)
    }

    // -- Maintenance --

    /// Rebuild search indexes (PostgreSQL specific).
    pub async fn rebuild_search_index(&self) -> AppResult<()> {
        // Note: REINDEX cannot be run inside a transaction block in some PG versions,
        // but sqlx execute might handle it.
        sqlx::query("REINDEX INDEX files_name_idx")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to rebuild index", e)
            })?;
        Ok(())
    }

    /// Find file records that are potentially orphaned (e.g. invalid storage_id).
    pub async fn find_orphaned_records(&self) -> AppResult<u64> {
        // For now, just count files with null storage_path or size 0 which might indicate issues
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM files WHERE storage_path IS NULL OR storage_path = ''",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to count orphaned records", e)
        })?;
        Ok(count as u64)
    }
}
