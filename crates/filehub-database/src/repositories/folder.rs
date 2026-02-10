//! Folder repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::folder::model::{CreateFolder, Folder};

/// Repository for folder CRUD and tree queries.
#[derive(Debug, Clone)]
pub struct FolderRepository {
    pool: PgPool,
}

impl FolderRepository {
    /// Create a new folder repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a folder by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Folder>> {
        sqlx::query_as::<_, Folder>("SELECT * FROM folders WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find folder", e))
    }

    /// Find a folder by storage ID and path.
    pub async fn find_by_path(&self, storage_id: Uuid, path: &str) -> AppResult<Option<Folder>> {
        sqlx::query_as::<_, Folder>("SELECT * FROM folders WHERE storage_id = $1 AND path = $2")
            .bind(storage_id)
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find folder by path", e)
            })
    }

    /// List root folders for a storage.
    pub async fn find_roots(&self, storage_id: Uuid) -> AppResult<Vec<Folder>> {
        sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders WHERE storage_id = $1 AND parent_id IS NULL ORDER BY name ASC",
        )
        .bind(storage_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list root folders", e))
    }

    /// List direct children of a folder.
    pub async fn find_children(
        &self,
        parent_id: Uuid,
        page: &PageRequest,
    ) -> AppResult<PageResponse<Folder>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE parent_id = $1")
            .bind(parent_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count children", e)
            })?;

        let folders = sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders WHERE parent_id = $1 ORDER BY name ASC LIMIT $2 OFFSET $3",
        )
        .bind(parent_id)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list children", e))?;

        Ok(PageResponse::new(
            folders,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Recursive query to get all descendants of a folder.
    pub async fn find_descendants(&self, parent_id: Uuid) -> AppResult<Vec<Folder>> {
        sqlx::query_as::<_, Folder>(
            "WITH RECURSIVE tree AS ( \
                SELECT * FROM folders WHERE id = $1 \
                UNION ALL \
                SELECT f.* FROM folders f INNER JOIN tree t ON f.parent_id = t.id \
             ) SELECT * FROM tree WHERE id != $1 ORDER BY depth ASC, name ASC",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list descendants", e))
    }

    /// Get the ancestor chain from a folder up to the root.
    pub async fn find_ancestors(&self, folder_id: Uuid) -> AppResult<Vec<Folder>> {
        sqlx::query_as::<_, Folder>(
            "WITH RECURSIVE ancestors AS ( \
                SELECT * FROM folders WHERE id = $1 \
                UNION ALL \
                SELECT f.* FROM folders f INNER JOIN ancestors a ON f.id = a.parent_id \
             ) SELECT * FROM ancestors ORDER BY depth ASC",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find ancestors", e))
    }

    /// Create a new folder.
    pub async fn create(&self, data: &CreateFolder) -> AppResult<Folder> {
        sqlx::query_as::<_, Folder>(
            "INSERT INTO folders (storage_id, parent_id, name, path, depth, owner_id) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(data.storage_id)
        .bind(data.parent_id)
        .bind(&data.name)
        .bind(&data.path)
        .bind(data.depth)
        .bind(data.owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("folders_storage_id_path_key") =>
            {
                AppError::conflict(format!("Folder path '{}' already exists", data.path))
            }
            _ => AppError::with_source(ErrorKind::Database, "Failed to create folder", e),
        })
    }

    /// Rename a folder.
    pub async fn rename(
        &self,
        folder_id: Uuid,
        new_name: &str,
        new_path: &str,
    ) -> AppResult<Folder> {
        sqlx::query_as::<_, Folder>(
            "UPDATE folders SET name = $2, path = $3, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(folder_id)
        .bind(new_name)
        .bind(new_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to rename folder", e))?
        .ok_or_else(|| AppError::not_found(format!("Folder {folder_id} not found")))
    }

    /// Move a folder to a new parent.
    pub async fn move_folder(
        &self,
        folder_id: Uuid,
        new_parent_id: Option<Uuid>,
        new_path: &str,
        new_depth: i32,
    ) -> AppResult<Folder> {
        sqlx::query_as::<_, Folder>(
            "UPDATE folders SET parent_id = $2, path = $3, depth = $4, updated_at = NOW() \
             WHERE id = $1 RETURNING *",
        )
        .bind(folder_id)
        .bind(new_parent_id)
        .bind(new_path)
        .bind(new_depth)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to move folder", e))?
        .ok_or_else(|| AppError::not_found(format!("Folder {folder_id} not found")))
    }

    /// Delete a folder (cascades to children and files).
    pub async fn delete(&self, folder_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM folders WHERE id = $1")
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to delete folder", e)
            })?;
        Ok(result.rows_affected() > 0)
    }

    /// Count files in a folder.
    pub async fn count_files(&self, folder_id: Uuid) -> AppResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE folder_id = $1")
            .bind(folder_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count files", e))?;
        Ok(count as u64)
    }

    /// Count child folders.
    pub async fn count_children(&self, folder_id: Uuid) -> AppResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE parent_id = $1")
            .bind(folder_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count children", e)
            })?;
        Ok(count as u64)
    }
}
