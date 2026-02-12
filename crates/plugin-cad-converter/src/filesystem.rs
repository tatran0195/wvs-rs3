//! Filesystem utilities for the conversion pipeline.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zip::ZipArchive;

use crate::error::ConversionError;
use crate::models::{ConversionInput, ConversionResult, FileType};

/// Filesystem utility functions.
pub struct FsUtils;

impl FsUtils {
    /// Maximum files in a ZIP archive.
    const MAX_ZIP_FILES: usize = 10_000;
    /// Maximum total extracted size (10 GB).
    const MAX_EXTRACTED_SIZE: u64 = 10 * 1024 * 1024 * 1024;
    /// Buffer size for ZIP copy.
    const BUFFER_SIZE: usize = 64 * 1024;

    /// Extract filename as String; returns `"unknown_file"` for empty paths.
    pub fn extract_filename_str(path: &Path) -> String {
        path.file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown_file".to_string())
    }

    /// Sanitize a filename stem for safe filesystem usage.
    pub fn sanitize_stem(filename: &str) -> String {
        let path = Path::new(filename);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);

        let sanitized: String = stem
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                    c
                } else if c.is_whitespace() {
                    '_'
                } else {
                    '\0'
                }
            })
            .filter(|c| *c != '\0')
            .take(200)
            .collect();

        if sanitized.is_empty() {
            "unnamed_file".to_string()
        } else {
            sanitized
        }
    }

    /// Generate unique filename: `[SanitizedStem]__[UUIDv7].[Extension]`.
    pub fn generate_unique_filename(original_name: &str, extension: &str) -> String {
        let stem = Self::sanitize_stem(original_name);
        let uuid = Uuid::now_v7().simple();
        format!("{}__{}.{}", stem, uuid, extension.trim_start_matches('.'))
    }

    /// Create a `ConversionResult` from a completed output file.
    pub async fn create_conversion_result(
        input: &ConversionInput,
        output_path: PathBuf,
        min_output_bytes: u64,
    ) -> Result<ConversionResult, ConversionError> {
        let metadata = tokio::fs::metadata(&output_path).await?;
        let size = metadata.len();

        if size < min_output_bytes {
            return Err(ConversionError::OutputEmpty { path: output_path });
        }

        let destination = output_path
            .parent()
            .ok_or_else(|| ConversionError::NoParentDir {
                path: output_path.clone(),
            })?
            .to_string_lossy()
            .to_string();

        Ok(ConversionResult {
            path: output_path.to_string_lossy().to_string(),
            filename: Self::extract_filename_str(&output_path),
            destination,
            original_name: input.original_name.clone(),
            size: size as f64,
        })
    }

    /// Handle a VTFx pass-through file (copy or move to output).
    pub async fn handle_vtfx_file(
        input_path: &Path,
        output_dir: &Path,
        should_delete_source: bool,
    ) -> Result<ConversionResult, ConversionError> {
        let original_name = Self::extract_filename_str(input_path);
        let out_name = Self::generate_unique_filename(&original_name, "vtfx");
        let out_path = output_dir.join(out_name);

        if should_delete_source {
            match tokio::fs::rename(input_path, &out_path).await {
                Ok(_) => {}
                Err(e) => {
                    // Cross-device or other rename failure — fall back to copy+delete
                    if e.kind() == std::io::ErrorKind::Other
                        || e.raw_os_error() == Some(17)
                        || e.raw_os_error() == Some(18)
                    {
                        tokio::fs::copy(input_path, &out_path).await?;
                        tokio::fs::remove_file(input_path).await?;
                    } else {
                        return Err(ConversionError::Io(e));
                    }
                }
            }
        } else {
            tokio::fs::copy(input_path, &out_path).await?;
        }

        let metadata = tokio::fs::metadata(&out_path).await?;
        let destination = output_dir.to_string_lossy().to_string();

        Ok(ConversionResult {
            path: out_path.to_string_lossy().to_string(),
            filename: Self::extract_filename_str(&out_path),
            destination,
            original_name,
            size: metadata.len() as f64,
        })
    }

    /// Extract a ZIP archive with security limits.
    pub fn extract_zip_file(
        zip_path: &Path,
        extract_to: &Path,
    ) -> Result<PathBuf, ConversionError> {
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        if archive.len() > Self::MAX_ZIP_FILES {
            return Err(ConversionError::ZipTooManyFiles {
                count: archive.len(),
                limit: Self::MAX_ZIP_FILES,
            });
        }

        let folder_name = format!("extract__{}", Uuid::now_v7().simple());
        let root_extract_dir = extract_to.join(folder_name);
        fs::create_dir_all(&root_extract_dir)?;

        let mut total_size = 0u64;

        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i)?;

            let enclosed_name = match zip_file.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => continue,
            };

            let out_path = root_extract_dir.join(&enclosed_name);

            total_size += zip_file.size();
            if total_size > Self::MAX_EXTRACTED_SIZE {
                // Clean up partially extracted directory
                let _ = fs::remove_dir_all(&root_extract_dir);
                return Err(ConversionError::ZipSizeExceeded {
                    limit: Self::MAX_EXTRACTED_SIZE,
                });
            }

            if zip_file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = File::create(&out_path)?;
                let mut buffer = vec![0u8; Self::BUFFER_SIZE];
                loop {
                    let n = zip_file.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    outfile.write_all(&buffer[..n])?;
                }
            }
        }

        Ok(root_extract_dir)
    }

    /// Find the primary input by name.
    pub fn find_primary_input<'a>(
        inputs: &'a [ConversionInput],
        primary_name: Option<&str>,
    ) -> Result<&'a ConversionInput, ConversionError> {
        let target = primary_name.ok_or(ConversionError::PrimaryNotSpecified)?;
        inputs
            .iter()
            .find(|i| Self::extract_filename_str(&i.path) == target)
            .ok_or_else(|| ConversionError::PrimaryNotFound {
                name: target.to_string(),
            })
    }

    /// Scan directory for supported file types.
    pub async fn get_supported_files_in_directory(
        dir_path: &Path,
        scan_deeper: bool,
    ) -> Result<Vec<PathBuf>, ConversionError> {
        let mut files = Vec::new();
        let mut dirs_to_visit = vec![dir_path.to_path_buf()];

        while let Some(current_dir) = dirs_to_visit.pop() {
            let mut entries = match tokio::fs::read_dir(&current_dir).await {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        dir = %current_dir.display(),
                        error = %e,
                        "Failed to read directory, skipping"
                    );
                    continue;
                }
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    if FileType::from_path_ref(&path).is_some() {
                        files.push(path);
                    }
                } else if path.is_dir() && scan_deeper {
                    dirs_to_visit.push(path);
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_stem_edge_cases() {
        assert_eq!(FsUtils::sanitize_stem("a b c.stp"), "a_b_c");
        assert_eq!(FsUtils::sanitize_stem("file<>:\"|?*.dwg"), "file");
        assert_eq!(FsUtils::sanitize_stem(""), "unnamed_file");

        // Test truncation at 200 chars
        let long_name = "a".repeat(300) + ".stp";
        let result = FsUtils::sanitize_stem(&long_name);
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn test_unique_filename_uniqueness() {
        let a = FsUtils::generate_unique_filename("test.stp", "vtfx");
        let b = FsUtils::generate_unique_filename("test.stp", "vtfx");
        assert_ne!(a, b);
    }
}
