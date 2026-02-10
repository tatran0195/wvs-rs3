//! Conversion orchestrator — coordinates the full conversion pipeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::id::FileId;

use crate::executor::{ConversionExecutor, ExecutionParams, ExecutionResult, ExecutorError};
use crate::formats::mapping::{
    CadFormat, ConversionMapping, ConversionMappingEntry, ConversionTarget,
};

/// Request to convert a CAD file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRequest {
    /// The file ID being converted
    pub file_id: FileId,
    /// Name of the original file
    pub file_name: String,
    /// Path to the source file in storage
    pub source_path: PathBuf,
    /// Target formats to produce
    pub targets: Vec<ConversionTarget>,
    /// Directory to write output files
    pub output_dir: PathBuf,
    /// Job ID for tracking
    pub job_id: String,
}

/// Result of a full conversion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// The file ID that was converted
    pub file_id: FileId,
    /// The job ID
    pub job_id: String,
    /// Whether the overall conversion succeeded
    pub success: bool,
    /// Results per target format
    pub target_results: Vec<TargetConversionResult>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Result for a single target format conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConversionResult {
    /// The target format
    pub target: ConversionTarget,
    /// Whether this specific conversion succeeded
    pub success: bool,
    /// Path to the output file (if successful)
    pub output_path: Option<PathBuf>,
    /// Output file size in bytes
    pub output_size: Option<u64>,
    /// MIME type of the output
    pub mime_type: String,
    /// Duration of this conversion in ms
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Orchestrates CAD file conversions
#[derive(Debug)]
pub struct CadConverter {
    /// Format mapping registry
    mapping: ConversionMapping,
    /// Command executor
    executor: ConversionExecutor,
}

impl CadConverter {
    /// Create a new CAD converter
    pub fn new(mapping: ConversionMapping, temp_dir: PathBuf) -> Self {
        Self {
            mapping,
            executor: ConversionExecutor::new(temp_dir),
        }
    }

    /// Check if a file name is a CAD file that can be converted
    pub fn is_convertible(&self, file_name: &str) -> bool {
        self.extract_extension(file_name)
            .map(|ext| self.mapping.is_cad_file(&ext))
            .unwrap_or(false)
    }

    /// Get the CAD format for a file name
    pub fn detect_format(&self, file_name: &str) -> Option<CadFormat> {
        let ext = self.extract_extension(file_name)?;
        CadFormat::from_extension(&ext)
    }

    /// Get supported output targets for a file
    pub fn supported_targets(&self, file_name: &str) -> Vec<ConversionTarget> {
        let ext = match self.extract_extension(file_name) {
            Some(e) => e,
            None => return Vec::new(),
        };

        self.mapping
            .get_mapping(&ext)
            .map(|m| m.targets.clone())
            .unwrap_or_default()
    }

    /// Execute a full conversion request
    pub async fn convert(&self, request: &ConversionRequest) -> ConversionResult {
        let start = std::time::Instant::now();
        let mut target_results = Vec::new();
        let mut errors = Vec::new();

        let ext = match self.extract_extension(&request.file_name) {
            Some(e) => e,
            None => {
                return ConversionResult {
                    file_id: request.file_id.clone(),
                    job_id: request.job_id.clone(),
                    success: false,
                    target_results: Vec::new(),
                    total_duration_ms: start.elapsed().as_millis() as u64,
                    errors: vec!["Could not determine file extension".to_string()],
                };
            }
        };

        let mapping_entry = match self.mapping.get_mapping(&ext) {
            Some(m) => m.clone(),
            None => {
                return ConversionResult {
                    file_id: request.file_id.clone(),
                    job_id: request.job_id.clone(),
                    success: false,
                    target_results: Vec::new(),
                    total_duration_ms: start.elapsed().as_millis() as u64,
                    errors: vec![format!(
                        "No conversion mapping found for extension '{}'",
                        ext
                    )],
                };
            }
        };

        let job_temp_dir = match self.executor.create_job_temp_dir(&request.job_id).await {
            Ok(d) => d,
            Err(e) => {
                return ConversionResult {
                    file_id: request.file_id.clone(),
                    job_id: request.job_id.clone(),
                    success: false,
                    target_results: Vec::new(),
                    total_duration_ms: start.elapsed().as_millis() as u64,
                    errors: vec![format!("Failed to create temp dir: {}", e)],
                };
            }
        };

        for target in &request.targets {
            if !mapping_entry.targets.contains(target) {
                tracing::warn!(
                    "Target format {:?} not supported for {}, skipping",
                    target,
                    ext
                );
                continue;
            }

            let output_filename = format!(
                "{}.{}",
                request
                    .source_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output"),
                target.extension()
            );
            let output_path = request.output_dir.join(&output_filename);

            let args = self.executor.substitute_args(
                &mapping_entry.args_template,
                &request.source_path,
                &output_path,
                target.extension(),
            );

            let params = ExecutionParams {
                command: mapping_entry.command.clone(),
                args,
                working_dir: Some(job_temp_dir.clone()),
                env_vars: std::collections::HashMap::new(),
                timeout_seconds: mapping_entry.timeout_seconds,
                input_path: request.source_path.clone(),
                output_path: output_path.clone(),
            };

            let target_start = std::time::Instant::now();

            match self.executor.execute(&params).await {
                Ok(result) => {
                    target_results.push(TargetConversionResult {
                        target: target.clone(),
                        success: result.success && result.output_path.is_some(),
                        output_path: result.output_path,
                        output_size: result.output_size,
                        mime_type: target.mime_type().to_string(),
                        duration_ms: target_start.elapsed().as_millis() as u64,
                        error: if result.success {
                            None
                        } else {
                            Some(result.stderr)
                        },
                    });
                }
                Err(e) => {
                    let err_msg = format!("Conversion to {} failed: {}", target, e);
                    tracing::error!("{}", err_msg);
                    errors.push(err_msg.clone());
                    target_results.push(TargetConversionResult {
                        target: target.clone(),
                        success: false,
                        output_path: None,
                        output_size: None,
                        mime_type: target.mime_type().to_string(),
                        duration_ms: target_start.elapsed().as_millis() as u64,
                        error: Some(err_msg),
                    });
                }
            }
        }

        if let Err(e) = self.executor.cleanup_job_temp_dir(&request.job_id).await {
            tracing::warn!(
                "Failed to cleanup temp dir for job {}: {}",
                request.job_id,
                e
            );
        }

        let any_success = target_results.iter().any(|r| r.success);

        ConversionResult {
            file_id: request.file_id.clone(),
            job_id: request.job_id.clone(),
            success: any_success,
            target_results,
            total_duration_ms: start.elapsed().as_millis() as u64,
            errors,
        }
    }

    /// Check which conversion tools are available on the system
    pub async fn check_available_tools(&self) -> Vec<ToolAvailability> {
        let mut results = Vec::new();
        let mut checked = std::collections::HashSet::new();

        for ext in self.mapping.supported_extensions() {
            if let Some(mapping) = self.mapping.get_mapping(&ext) {
                if checked.contains(&mapping.command) {
                    continue;
                }
                checked.insert(mapping.command.clone());

                let available = self
                    .executor
                    .check_command_available(&mapping.command)
                    .await;

                results.push(ToolAvailability {
                    command: mapping.command.clone(),
                    available,
                    formats: vec![ext],
                });
            }
        }

        results
    }

    /// Get the mapping registry
    pub fn mapping(&self) -> &ConversionMapping {
        &self.mapping
    }

    /// Extract the file extension from a filename
    fn extract_extension(&self, file_name: &str) -> Option<String> {
        Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
    }
}

/// Information about a conversion tool's availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAvailability {
    /// The command name
    pub command: String,
    /// Whether the command is available
    pub available: bool,
    /// Formats this tool handles
    pub formats: Vec<String>,
}
