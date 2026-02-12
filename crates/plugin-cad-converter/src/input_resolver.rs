//! Input resolution: expands ZIPs, scans directories, produces ConversionInput list.

use std::path::PathBuf;

use tracing::warn;

use crate::error::ConversionError;
use crate::filesystem::FsUtils;
use crate::models::{ConversionInput, FileType};

/// Resolves raw input paths into typed ConversionInput items.
pub struct InputResolver {
    /// Working directory for ZIP extraction.
    job_dir: PathBuf,
    /// Whether to scan subdirectories recursively.
    scan_deeper: bool,
    /// Tracked source paths for cleanup.
    source_cleanup_list: Vec<PathBuf>,
    /// Tracked extraction directories for cleanup.
    extraction_dirs: Vec<PathBuf>,
}

impl InputResolver {
    /// Create a new resolver.
    pub fn new(job_dir: PathBuf, scan_deeper: bool) -> Self {
        Self {
            job_dir,
            scan_deeper,
            source_cleanup_list: Vec::new(),
            extraction_dirs: Vec::new(),
        }
    }

    /// Resolve input path strings into typed ConversionInput items.
    pub async fn resolve_inputs(
        &mut self,
        inputs: Vec<String>,
    ) -> Result<Vec<ConversionInput>, ConversionError> {
        let mut results = Vec::new();

        for input_str in inputs {
            let path = PathBuf::from(&input_str);
            if !path.exists() {
                warn!(path = %input_str, "Input path does not exist, skipping");
                continue;
            }

            self.source_cleanup_list.push(path.clone());

            if path.is_file() {
                let ftype = FileType::from_path_ref(&path).unwrap_or(FileType::Unknown);
                if ftype.is_archive_format() {
                    let extracted = self.handle_zip(&path).await?;
                    results.extend(extracted);
                } else {
                    results.push(ConversionInput {
                        path: path.clone(),
                        original_name: FsUtils::extract_filename_str(&path),
                        file_type: ftype,
                    });
                }
            } else if path.is_dir() {
                let scanned = self.scan_directory(&path).await?;
                results.extend(scanned);
            }
        }

        Ok(results)
    }

    /// Extract ZIP and scan contents.
    async fn handle_zip(
        &mut self,
        zip_path: &PathBuf,
    ) -> Result<Vec<ConversionInput>, ConversionError> {
        let zp = zip_path.clone();
        let jd = self.job_dir.clone();

        let extract_dir =
            tokio::task::spawn_blocking(move || FsUtils::extract_zip_file(&zp, &jd)).await??;

        // Track the extraction directory for cleanup
        self.extraction_dirs.push(extract_dir.clone());

        self.scan_directory(&extract_dir).await
    }

    /// Scan directory for supported files.
    async fn scan_directory(&self, dir: &PathBuf) -> Result<Vec<ConversionInput>, ConversionError> {
        let files = FsUtils::get_supported_files_in_directory(dir, self.scan_deeper).await?;

        Ok(files
            .into_iter()
            .map(|p| ConversionInput {
                original_name: FsUtils::extract_filename_str(&p),
                file_type: FileType::from_path_ref(&p).unwrap_or(FileType::Unknown),
                path: p,
            })
            .collect())
    }

    /// Clean up original source paths.
    pub async fn cleanup_sources(&self) {
        for path in &self.source_cleanup_list {
            if path.is_file() {
                let _ = tokio::fs::remove_file(path).await;
            } else if path.is_dir() {
                let _ = tokio::fs::remove_dir_all(path).await;
            }
        }
    }

    /// Clean up extraction directories.
    pub async fn cleanup_extractions(&self) {
        for dir in &self.extraction_dirs {
            let _ = tokio::fs::remove_dir_all(dir).await;
        }
    }

    /// Get tracked source paths.
    pub fn source_cleanup_list(&self) -> &[PathBuf] {
        &self.source_cleanup_list
    }

    /// Get tracked extraction directories.
    pub fn extraction_dirs(&self) -> &[PathBuf] {
        &self.extraction_dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_skips_nonexistent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut resolver = InputResolver::new(temp.path().to_path_buf(), false);
        let results = resolver
            .resolve_inputs(vec!["/nonexistent/file.stp".to_string()])
            .await
            .expect("ok");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_extraction_dirs_tracked() {
        let resolver = InputResolver::new(PathBuf::from("/tmp"), false);
        assert!(resolver.extraction_dirs().is_empty());
    }
}
