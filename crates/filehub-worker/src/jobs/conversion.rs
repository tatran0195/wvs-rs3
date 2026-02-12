//! CAD conversion job handler.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Trait for conversion execution — decouples from plugin-cad-converter
#[async_trait]
pub trait ConversionService: Send + Sync + std::fmt::Debug {
    /// Execute a conversion job
    async fn convert(
        &self,
        file_id: Uuid,
        file_name: &str,
        source_path: &str,
        targets: &[String],
        output_dir: &str,
        job_id: &str,
    ) -> Result<Value, AppError>;
}

/// Handles CAD conversion jobs
#[derive(Debug)]
pub struct CadConversionJobHandler {
    /// Conversion service
    converter: Arc<dyn ConversionService>,
}

impl CadConversionJobHandler {
    /// Create a new CAD conversion job handler
    pub fn new(converter: Arc<dyn ConversionService>) -> Self {
        Self { converter }
    }
}

#[async_trait]
impl JobHandler for CadConversionJobHandler {
    fn job_type(&self) -> &str {
        "cad_conversion"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let file_id_str = job
            .payload
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                JobExecutionError::Permanent("Missing file_id in conversion payload".to_string())
            })?;

        let file_id = Uuid::parse_str(file_id_str)
            .map_err(|e| JobExecutionError::Permanent(format!("Invalid file_id: {}", e)))?;

        let file_name = job
            .payload
            .get("file_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                JobExecutionError::Permanent("Missing file_name in conversion payload".to_string())
            })?;

        let source_path = job
            .payload
            .get("source_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                JobExecutionError::Permanent(
                    "Missing source_path in conversion payload".to_string(),
                )
            })?;

        let targets: Vec<String> = job
            .payload
            .get("targets")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let output_dir = job
            .payload
            .get("output_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("./data/cache/conversions");

        let job_id = job.id.to_string();

        tracing::info!(
            "Starting CAD conversion: file='{}', targets={:?}",
            file_name,
            targets
        );

        let result = self
            .converter
            .convert(
                file_id,
                file_name,
                source_path,
                &targets,
                output_dir,
                &job_id,
            )
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Conversion failed: {}", e)))?;

        tracing::info!("CAD conversion completed for file '{}'", file_name);

        Ok(Some(result))
    }
}
