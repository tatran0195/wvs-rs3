//! Conversion processor: pipeline orchestration with timeout, retry,
//! cancellation, output validation, and metrics collection.

use crate::config::ConversionConfig;
use crate::error::ConversionError;
use crate::filesystem::FsUtils;
use crate::input_resolver::InputResolver;
use crate::metrics::ConversionMetrics;
use crate::models::*;
use crate::scripting::ScriptingEngine;

use futures::future::try_join_all;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// The main conversion processor.
#[derive(Debug, Clone)]
pub struct ConversionProcessor {
    /// Root directory for temporary working directories.
    temp_root: PathBuf,
    /// Plugin configuration.
    config: ConversionConfig,
    /// Global semaphore limiting concurrent Jupiter processes.
    global_limiter: Arc<Semaphore>,
    /// Conversion metrics collector.
    metrics: Arc<ConversionMetrics>,
}

impl ConversionProcessor {
    /// Create a new processor.
    pub fn new(config: ConversionConfig) -> Result<Self, ConversionError> {
        let temp_root = config.effective_temp_root();
        std::fs::create_dir_all(&temp_root)?;

        Ok(Self {
            global_limiter: Arc::new(Semaphore::new(config.max_global_concurrency)),
            metrics: Arc::new(ConversionMetrics::new()),
            config,
            temp_root,
        })
    }

    /// Execute a conversion job with cancellation support.
    #[instrument(skip(self, inputs, cancel), fields(job_id))]
    pub async fn execute_job(
        &self,
        inputs: Vec<String>,
        output_dir: String,
        options: Option<ConversionOptions>,
        cancel: CancellationToken,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        let options = options.unwrap_or_default();
        let job_id = Uuid::now_v7();
        tracing::Span::current().record("job_id", job_id.to_string());

        let output_path = PathBuf::from(&output_dir);
        let job_path = self.temp_root.join(job_id.simple().to_string());

        tokio::fs::create_dir_all(&output_path).await?;
        tokio::fs::create_dir_all(&job_path).await?;

        // Run pipeline; clean up job directory regardless of outcome
        let result = self
            .run_pipeline(inputs, &output_path, &job_path, &options, cancel)
            .await;

        // Synchronous cleanup of job directory (best-effort)
        if let Err(e) = tokio::fs::remove_dir_all(&job_path).await {
            warn!(
                job_dir = %job_path.display(),
                error = %e,
                "Failed to clean up job directory"
            );
        }

        result
    }

    /// Execute job without cancellation (convenience wrapper).
    pub async fn execute_job_simple(
        &self,
        inputs: Vec<String>,
        output_dir: String,
        options: Option<ConversionOptions>,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        self.execute_job(inputs, output_dir, options, CancellationToken::new())
            .await
    }

    /// Core pipeline.
    async fn run_pipeline(
        &self,
        inputs: Vec<String>,
        output_path: &Path,
        job_path: &Path,
        options: &ConversionOptions,
        cancel: CancellationToken,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        // Check cancellation before starting
        if cancel.is_cancelled() {
            return Err(ConversionError::Cancelled);
        }

        // Phase 1: Resolve inputs
        let mut resolver = InputResolver::new(job_path.to_path_buf(), options.should_scan_deeper());
        let all_inputs = resolver.resolve_inputs(inputs).await?;

        if all_inputs.is_empty() {
            info!("No processable files found in inputs");
            return Ok(Vec::new());
        }

        // Phase 2: Partition
        let (vtfx, cad): (Vec<_>, Vec<_>) = all_inputs
            .into_iter()
            .partition(|i| i.file_type.is_vtfx_format());

        let mut results = Vec::new();

        // Phase 3: VTFx pass-through
        if !vtfx.is_empty() {
            if cancel.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }
            info!(count = vtfx.len(), "Processing VTFx pass-through files");
            let vtfx_results = self
                .process_vtfx(vtfx, output_path, options.should_delete_source())
                .await?;
            results.extend(vtfx_results);
        }

        // Phase 4: CAD/FEA conversion
        if !cad.is_empty() {
            if cancel.is_cancelled() {
                return Err(ConversionError::Cancelled);
            }
            info!(count = cad.len(), "Processing CAD/FEA files");
            let cad_results = self
                .process_cad(cad, output_path, job_path, options, cancel.clone())
                .await?;
            results.extend(cad_results);
        }

        // Phase 5: Source cleanup
        if options.should_delete_source() {
            resolver.cleanup_sources().await;
        }

        // Always clean up extraction directories
        resolver.cleanup_extractions().await;

        info!(total_outputs = results.len(), "Conversion job completed");
        Ok(results)
    }

    /// VTFx pass-through with IO concurrency.
    async fn process_vtfx(
        &self,
        inputs: Vec<ConversionInput>,
        output_path: &Path,
        delete_source: bool,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        let sem = Arc::new(Semaphore::new(self.config.max_io_concurrency));
        let op = output_path.to_owned();
        let metrics = Arc::clone(&self.metrics);

        let tasks = inputs.into_iter().map(|input| {
            let s = sem.clone();
            let o = op.clone();
            let m = metrics.clone();
            async move {
                let _permit = s
                    .acquire()
                    .await
                    .map_err(|_| ConversionError::SemaphoreClosed {
                        reason: "IO semaphore closed".to_string(),
                    })?;
                let result = FsUtils::handle_vtfx_file(&input.path, &o, delete_source).await?;
                m.record_vtfx_passthrough(result.size as u64);
                Ok(result)
            }
        });

        try_join_all(tasks).await
    }

    /// CAD/FEA dispatch by mode.
    async fn process_cad(
        &self,
        inputs: Vec<ConversionInput>,
        output_path: &Path,
        job_path: &Path,
        options: &ConversionOptions,
        cancel: CancellationToken,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        match options.conversion_mode() {
            ConversionMode::Single => {
                self.process_cad_single(inputs, output_path, job_path, options, cancel)
                    .await
            }
            ConversionMode::Assembly | ConversionMode::Combine => {
                self.process_cad_multi(
                    inputs,
                    output_path,
                    job_path,
                    options,
                    options.conversion_mode(),
                    cancel,
                )
                .await
            }
        }
    }

    /// Single mode: parallel independent conversions.
    async fn process_cad_single(
        &self,
        inputs: Vec<ConversionInput>,
        output_path: &Path,
        job_path: &Path,
        options: &ConversionOptions,
        cancel: CancellationToken,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        let req_limit = options.concurrency().clamp(1, 4) as usize;
        let req_sem = Arc::new(Semaphore::new(req_limit));

        let mut tasks: Vec<tokio::task::JoinHandle<Result<ConversionResult, ConversionError>>> =
            Vec::with_capacity(inputs.len());

        for input in inputs {
            let proc = self.clone();
            let i = input.clone();
            let o = output_path.to_owned();
            let j = job_path.to_owned();
            let rs = req_sem.clone();
            let c = cancel.clone();

            tasks.push(tokio::spawn(async move {
                if c.is_cancelled() {
                    return Err(ConversionError::Cancelled);
                }

                let _l = rs
                    .acquire()
                    .await
                    .map_err(|_| ConversionError::SemaphoreClosed {
                        reason: "request semaphore".to_string(),
                    })?;

                let _g = proc.global_limiter.acquire().await.map_err(|_| {
                    ConversionError::SemaphoreClosed {
                        reason: "global semaphore".to_string(),
                    }
                })?;

                if c.is_cancelled() {
                    return Err(ConversionError::Cancelled);
                }

                proc.convert_single_input(&i, &o, &j, c).await
            }));
        }

        let mut results = Vec::new();
        let join_results = try_join_all(tasks).await?;
        for task_result in join_results {
            results.push(task_result?);
        }

        Ok(results)
    }

    /// Assembly/Combine mode.
    async fn process_cad_multi(
        &self,
        inputs: Vec<ConversionInput>,
        output_path: &Path,
        job_path: &Path,
        options: &ConversionOptions,
        mode: ConversionMode,
        cancel: CancellationToken,
    ) -> Result<Vec<ConversionResult>, ConversionError> {
        let _permit =
            self.global_limiter
                .acquire()
                .await
                .map_err(|_| ConversionError::AtCapacity {
                    max_slots: self.config.max_global_concurrency,
                })?;

        if cancel.is_cancelled() {
            return Err(ConversionError::Cancelled);
        }

        let primary_name = options.get_primary_name()?;
        let primary = FsUtils::find_primary_input(&inputs, Some(&primary_name))?;

        let out_name = FsUtils::generate_unique_filename(&primary.original_name, "vtfx");
        let out_path = output_path.join(out_name);

        let script = ScriptingEngine::generate_python_script(
            &inputs,
            &out_path,
            mode,
            Some(primary),
            job_path,
        )
        .await?;

        let exec_result = self.execute_jupiter_with_retry(&script, cancel).await;

        // Always clean up script
        let _ = tokio::fs::remove_file(&script).await;

        exec_result?;

        self.validate_output(&out_path)?;

        Ok(vec![
            FsUtils::create_conversion_result(primary, out_path, self.config.min_output_bytes)
                .await?,
        ])
    }

    /// Convert a single input file.
    async fn convert_single_input(
        &self,
        input: &ConversionInput,
        output_dir: &Path,
        job_path: &Path,
        cancel: CancellationToken,
    ) -> Result<ConversionResult, ConversionError> {
        self.metrics.record_started();
        let start = Instant::now();

        let out_name = FsUtils::generate_unique_filename(&input.original_name, "vtfx");
        let out_path = output_dir.join(out_name);

        let script = ScriptingEngine::generate_python_script(
            &[input.clone()],
            &out_path,
            ConversionMode::Single,
            None,
            job_path,
        )
        .await?;

        let exec_result = self.execute_jupiter_with_retry(&script, cancel).await;

        let _ = tokio::fs::remove_file(&script).await;

        match exec_result {
            Ok(()) => {
                self.validate_output(&out_path)?;
                let result = FsUtils::create_conversion_result(
                    input,
                    out_path,
                    self.config.min_output_bytes,
                )
                .await?;

                let duration = start.elapsed();
                self.metrics.record_success(duration, result.size as u64);

                Ok(result)
            }
            Err(e) => {
                self.metrics.record_failure();
                Err(e)
            }
        }
    }

    /// Execute Jupiter with retry logic.
    async fn execute_jupiter_with_retry(
        &self,
        script_path: &Path,
        cancel: CancellationToken,
    ) -> Result<(), ConversionError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if cancel.is_cancelled() {
                self.metrics.record_cancelled();
                return Err(ConversionError::Cancelled);
            }

            if attempt > 0 {
                info!(
                    attempt = attempt,
                    max = self.config.max_retries,
                    "Retrying Jupiter execution"
                );
                tokio::time::sleep(Duration::from_secs(self.config.retry_delay_seconds)).await;
            }

            match self.execute_jupiter(script_path, cancel.clone()).await {
                Ok(()) => return Ok(()),
                Err(ConversionError::Cancelled) => {
                    self.metrics.record_cancelled();
                    return Err(ConversionError::Cancelled);
                }
                Err(ConversionError::JupiterTimeout { .. }) => {
                    self.metrics.record_timeout();
                    // Timeout is not retryable
                    return Err(ConversionError::JupiterTimeout {
                        timeout_seconds: self.config.jupiter_timeout_seconds,
                    });
                }
                Err(e) => {
                    warn!(
                        attempt = attempt,
                        error = %e,
                        "Jupiter execution failed"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(ConversionError::JupiterKilled))
    }

    /// Execute Jupiter with timeout and cancellation.
    async fn execute_jupiter(
        &self,
        script_path: &Path,
        cancel: CancellationToken,
    ) -> Result<(), ConversionError> {
        let script_str = script_path
            .to_str()
            .ok_or_else(|| ConversionError::InvalidUtf8Path {
                path: script_path.to_path_buf(),
            })?;

        if !self.config.jupiter_path.exists() {
            return Err(ConversionError::JupiterNotFound {
                path: self.config.jupiter_path.clone(),
            });
        }

        let mut cmd = tokio::process::Command::new(&self.config.jupiter_path);

        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let (stdout_cfg, stderr_cfg) = if self.config.capture_output {
            (std::process::Stdio::piped(), std::process::Stdio::piped())
        } else {
            (std::process::Stdio::null(), std::process::Stdio::null())
        };

        cmd.args(["-b", "-py", script_str, "-keywebapp"])
            .stdout(stdout_cfg)
            .stderr(stderr_cfg)
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);

        debug!(
            jupiter = %self.config.jupiter_path.display(),
            script = %script_path.display(),
            timeout_s = self.config.jupiter_timeout_seconds,
            "Spawning Jupiter process"
        );

        let start = Instant::now();

        let mut child = cmd.spawn()?;

        // Take stdout/stderr handles before the select block
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let timeout = Duration::from_secs(self.config.jupiter_timeout_seconds);

        // Race: process completion vs timeout vs cancellation
        tokio::select! {
            result = child.wait() => {
                let status = result?;
                let elapsed = start.elapsed();

                // Read stdout and stderr if they were captured
                let stdout_str = if let Some(mut out) = stdout {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = out.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                } else {
                    String::new()
                };

                let stderr_str = if let Some(mut err) = stderr {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = err.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                } else {
                    String::new()
                };

                if !stderr_str.is_empty() {
                    debug!(stderr = %stderr_str, "Jupiter stderr output");
                }

                if status.success() {
                    info!(
                        elapsed_ms = elapsed.as_millis() as u64,
                        "Jupiter conversion completed"
                    );
                    Ok(())
                } else {
                    let code = status.code().unwrap_or(-1);
                    error!(
                        code = code,
                        elapsed_ms = elapsed.as_millis() as u64,
                        stderr = %stderr_str,
                        "Jupiter failed"
                    );
                    Err(ConversionError::JupiterFailed {
                        code,
                        stderr: stderr_str,
                        stdout: stdout_str,
                    })
                }
            }
            _ = tokio::time::sleep(timeout) => {
                error!(
                    timeout_s = self.config.jupiter_timeout_seconds,
                    "Jupiter process timed out, killing"
                );
                let _ = child.kill().await;
                Err(ConversionError::JupiterTimeout {
                    timeout_seconds: self.config.jupiter_timeout_seconds,
                })
            }
            _ = cancel.cancelled() => {
                info!("Conversion cancelled, killing Jupiter process");
                let _ = child.kill().await;
                Err(ConversionError::Cancelled)
            }
        }
    }

    /// Validate that the output file exists and meets minimum size.
    fn validate_output(&self, output_path: &Path) -> Result<(), ConversionError> {
        if !output_path.exists() {
            return Err(ConversionError::OutputNotCreated {
                path: output_path.to_path_buf(),
            });
        }

        // Synchronous metadata check (file was just created, should be fast)
        let metadata = std::fs::metadata(output_path)?;
        if metadata.len() < self.config.min_output_bytes {
            return Err(ConversionError::OutputEmpty {
                path: output_path.to_path_buf(),
            });
        }

        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &ConversionConfig {
        &self.config
    }

    /// Get available global conversion slots.
    pub fn available_global_slots(&self) -> usize {
        self.global_limiter.available_permits()
    }

    /// Get the metrics collector.
    pub fn metrics(&self) -> &ConversionMetrics {
        &self.metrics
    }

    /// Get a metrics snapshot.
    pub fn metrics_snapshot(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let config = ConversionConfig {
            temp_root: Some(std::env::temp_dir().join("filehub_test_processor_v2")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        assert_eq!(processor.available_global_slots(), 4);
    }

    #[test]
    fn test_validate_output_nonexistent() {
        let config = ConversionConfig {
            temp_root: Some(std::env::temp_dir().join("filehub_validate_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let result = processor.validate_output(Path::new("/nonexistent/file.vtfx"));
        assert!(matches!(
            result,
            Err(ConversionError::OutputNotCreated { .. })
        ));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let config = ConversionConfig {
            temp_root: Some(std::env::temp_dir().join("filehub_cancel_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = processor
            .execute_job(
                vec!["/some/file.stp".to_string()],
                "/some/output".to_string(),
                None,
                cancel,
            )
            .await;

        assert!(matches!(result, Err(ConversionError::Cancelled)));
    }

    #[test]
    fn test_metrics_accessible() {
        let config = ConversionConfig {
            temp_root: Some(std::env::temp_dir().join("filehub_metrics_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let snap = processor.metrics_snapshot();
        assert_eq!(snap.conversions_started, 0);
    }
}
