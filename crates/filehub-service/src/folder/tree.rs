//! Folder tree building and path resolution.

use std::sync::Arc;

use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_database::repositories::folder::FolderRepository;
use filehub_entity::folder::{Folder, FolderNode};

use crate::context::RequestContext;

/// Builds folder trees and resolves paths.
#[derive(Debug, Clone)]
pub struct TreeService {
    /// Folder repository.
    folder_repo: Arc<FolderRepository>,
}

impl TreeService {
    /// Creates a new tree service.
    pub fn new(folder_repo: Arc<FolderRepository>) -> Self {
        Self { folder_repo }
    }

    /// Builds the complete folder tree starting from a root folder.
    pub async fn get_tree(
        &self,
        ctx: &RequestContext,
        folder_id: Uuid,
    ) -> Result<FolderNode, AppError> {
        let root = self
            .folder_repo
            .find_by_id(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Folder not found"))?;

        let descendants = self
            .folder_repo
            .find_descendants(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get descendants: {e}")))?;

        // Fetch file counts
        let mut ids = vec![folder_id];
        ids.extend(descendants.iter().map(|f| f.id));
        let file_counts = self
            .folder_repo
            .count_files_batch(&ids)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get file counts: {e}")))?;

        let tree = self.build_tree(root, &descendants, &file_counts);
        Ok(tree)
    }

    /// Builds a tree from a flat list of folders.
    fn build_tree(
        &self,
        root: Folder,
        all_folders: &[Folder],
        file_counts: &std::collections::HashMap<Uuid, u64>,
    ) -> FolderNode {
        let children: Vec<FolderNode> = all_folders
            .iter()
            .filter(|f| f.parent_id == Some(root.id))
            .map(|child| self.build_tree(child.clone(), all_folders, file_counts))
            .collect();

        FolderNode {
            id: root.id,
            name: root.name,
            path: root.path,
            depth: root.depth,
            child_count: children.len() as u64,
            file_count: *file_counts.get(&root.id).unwrap_or(&0),
            children,
        }
    }

    /// Resolves a path string to a folder ID.
    pub async fn resolve_path(
        &self,
        storage_id: Uuid,
        path: &str,
    ) -> Result<Option<Folder>, AppError> {
        self.folder_repo
            .find_by_path(storage_id, path)
            .await
            .map_err(|e| AppError::internal(format!("Path resolution failed: {e}")))
    }

    /// Gets the breadcrumb trail from root to the given folder.
    pub async fn get_breadcrumbs(&self, folder_id: Uuid) -> Result<Vec<Folder>, AppError> {
        let ancestry_ids = self
            .folder_repo
            .get_ancestry(folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Ancestry lookup failed: {e}")))?;

        let mut breadcrumbs = Vec::new();
        for id in ancestry_ids.iter().rev() {
            if let Some(folder) = self
                .folder_repo
                .find_by_id(*id)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            {
                breadcrumbs.push(folder);
            }
        }

        Ok(breadcrumbs)
    }
}
