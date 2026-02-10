//! Folder tree structures for hierarchical display.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A node in a folder tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderNode {
    /// Folder ID.
    pub id: Uuid,
    /// Folder name.
    pub name: String,
    /// Full path.
    pub path: String,
    /// Depth level.
    pub depth: i32,
    /// Number of child folders.
    pub child_count: u64,
    /// Number of files in this folder.
    pub file_count: u64,
    /// Child folder nodes.
    pub children: Vec<FolderNode>,
}

/// A complete folder tree rooted at a specific folder or storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTree {
    /// The root node(s) of the tree.
    pub roots: Vec<FolderNode>,
    /// Total number of folders in the tree.
    pub total_folders: u64,
}

impl FolderTree {
    /// Create an empty folder tree.
    pub fn empty() -> Self {
        Self {
            roots: Vec::new(),
            total_folders: 0,
        }
    }
}
