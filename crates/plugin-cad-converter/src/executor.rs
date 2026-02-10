//! CMD script execution for CAD file conversions.
//!
//! Executes external conversion tools as child processes with
//! timeout management and output capturing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tracing;

/// Errors from conversion execution
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The conversion command was not found
    #[error("Conversion command not found: {0}")]
    CommandNotFound(String),

    /// The conversion process timed out
    #[error("Conversion timed out after {0} seconds")]
    Timeout(u64),

    /// The conversion process exited with a non-zero code
    #[error("Conversion failed with exit code {code}: {stderr}")]
    ProcessFailed {
        /// The exit code
        code: i32,
        /// Standard error output
        stderr: String,
    },

    /// IO error during conversion
    #[error("IO error during conversion: {0}")]
    IoError(#[from] std::io::Error),

    /// Output file was not created
    #[error("Expected output file not created: {0}")]
    OutputMissing(String),

    /// Invalid conversion parameters
    #[error("Invalid conversion parameters: {0}")]
    InvalidParams(String),
}

/// Result of a conversion execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the conversion succeeded
    pub success: bool,
    /// Path to the output file
    pub output_path: Option<PathBuf>,
    /// Standard output from the process
    pub stdout: String,
    /// Standard error from the process
    pub stderr: String,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Duration of the conversion
    pub duration_ms: u64,
    /// Output file size in bytes
    pub output_size: Option<u64>,
}

/// Parameters for executing a conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    /// The command to execute
    pub command: String,
    /// Arguments (after placeholder substitution)
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Path to the input file
    pub input_path: PathBuf,
    /// Path where output should be written
    pub output_path: PathBuf,
}

/// Executor for running external conversion commands
#[derive(Debug, Clone)]
pub struct ConversionExecutor {
    /// Base directory for temporary conversion files
    temp_dir: PathBuf,
}

impl ConversionExecutor {
    /// Create a new conversion executor
    pub fn new(temp_dir: PathBuf) -> Self {
        Self { temp_dir }
    }

    /// Substitute template placeholders in arguments
    pub fn substitute_args(
        &self,
        template_args: &[String],
        input_path: &Path,
        output_path: &Path,
        format: &str,
    ) -> Vec<String> {
        let input_str = input_path.to_string_lossy();
        let output_str = output_path.to_string_lossy();
        let input_dir = input_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let output_dir = output_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        template_args
            .iter()
            .map(|arg| {
                arg.replace("{input}", &input_str)
                    .replace("{output}", &output_str)
                    .replace("{format}", format)
                    .replace("{input_dir}", &input_dir)
                    .replace("{output_dir}", &output_dir)
            })
            .collect()
    }

    /// Execute a conversion command
    pub async fn execute(
        &self,
        params: &ExecutionParams,
    ) -> Result<ExecutionResult, ExecutorError> {
        let start = std::time::Instant::now();

        tracing::info!(
            "Executing conversion: command='{}', args={:?}, input='{}', output='{}'",
            params.command,
            params.args,
            params.input_path.display(),
            params.output_path.display()
        );

        if let Some(parent) = params.output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut cmd = Command::new(&params.command);
        cmd.args(&params.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref dir) = params.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &params.env_vars {
            cmd.env(key, value);
        }

        let timeout = Duration::from_secs(params.timeout_seconds);

        let result = tokio::time::timeout(timeout, cmd.output()).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                if !output.status.success() {
                    let code = exit_code.unwrap_or(-1);
                    tracing::error!(
                        "Conversion failed: command='{}', exit_code={}, stderr='{}'",
                        params.command,
                        code,
                        stderr.chars().take(500).collect::<String>()
                    );
                    return Err(ExecutorError::ProcessFailed {
                        code,
                        stderr: stderr.chars().take(2000).collect(),
                    });
                }

                let output_size = if params.output_path.exists() {
                    tokio::fs::metadata(&params.output_path)
                        .await
                        .ok()
                        .map(|m| m.len())
                } else {
                    None
                };

                if output_size.is_none() {
                    tracing::warn!(
                        "Conversion command succeeded but output file not found: '{}'",
                        params.output_path.display()
                    );
                }

                tracing::info!(
                    "Conversion completed: command='{}', duration={}ms, output_size={:?}",
                    params.command,
                    duration_ms,
                    output_size
                );

                Ok(ExecutionResult {
                    success: true,
                    output_path: if output_size.is_some() {
                        Some(params.output_path.clone())
                    } else {
                        None
                    },
                    stdout,
                    stderr,
                    exit_code,
                    duration_ms,
                    output_size,
                })
            }
            Ok(Err(e)) => {
                tracing::error!(
                    "Failed to execute conversion command '{}': {}",
                    params.command,
                    e
                );
                Err(ExecutorError::IoError(e))
            }
            Err(_) => {
                tracing::error!(
                    "Conversion timed out after {}s: command='{}'",
                    params.timeout_seconds,
                    params.command
                );
                Err(ExecutorError::Timeout(params.timeout_seconds))
            }
        }
    }

    /// Check if a conversion command is available on the system
    pub async fn check_command_available(&self, command: &str) -> bool {
        let result = if cfg!(target_os = "windows") {
            Command::new("where")
                .arg(command)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
        } else {
            Command::new("which")
                .arg(command)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
        };

        match result {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Get the temporary directory
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Create a temporary directory for a conversion job
    pub async fn create_job_temp_dir(&self, job_id: &str) -> Result<PathBuf, ExecutorError> {
        let dir = self.temp_dir.join("cad_conversion").join(job_id);
        tokio::fs::create_dir_all(&dir).await?;
        Ok(dir)
    }

    /// Clean up a job's temporary directory
    pub async fn cleanup_job_temp_dir(&self, job_id: &str) -> Result<(), ExecutorError> {
        let dir = self.temp_dir.join("cad_conversion").join(job_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }
}
