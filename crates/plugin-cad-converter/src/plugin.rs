//! CAD converter plugin implementation — integrates with FileHub plugin system.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing;

use filehub_core::error::AppError;
use filehub_core::types::id::FileId;
use filehub_plugin::hooks::definitions::{
    HookAction, HookContext, HookHandler, HookPayload, HookResult,
};
use filehub_plugin::hooks::registry::HookRegistry;
use filehub_plugin::registry::PluginInfo;

use crate::converter::{CadConverter, ConversionRequest};
use crate::formats::mapping::{ConversionMapping, ConversionTarget};

/// CAD converter plugin for FileHub
#[derive(Debug)]
pub struct CadConverterPlugin {
    /// Plugin information
    info: PluginInfo,
    /// The CAD converter instance
    converter: Option<Arc<CadConverter>>,
    /// Output directory for converted files
    output_dir: PathBuf,
}

impl CadConverterPlugin {
    /// Create a new CAD converter plugin
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "cad-converter".to_string(),
                version: "1.0.0".to_string(),
                description: "CAD file format conversion".to_string(),
                author: "Suzuki FileHub".to_string(),
            },
            converter: None,
            output_dir: PathBuf::from("./data/cache/conversions"),
        }
    }

    /// Initialize the plugin with configuration
    pub async fn initialize(
        &mut self,
        temp_dir: PathBuf,
        output_dir: PathBuf,
        custom_mappings: Option<ConversionMapping>,
    ) -> Result<Arc<CadConverter>, AppError> {
        let mapping = custom_mappings.unwrap_or_default();

        self.output_dir = output_dir;

        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create output dir: {}", e)))?;

        let converter = Arc::new(CadConverter::new(mapping, temp_dir));

        let tools = converter.check_available_tools().await;
        for tool in &tools {
            if tool.available {
                tracing::info!(
                    "CAD conversion tool available: '{}' (formats: {:?})",
                    tool.command,
                    tool.formats
                );
            } else {
                tracing::warn!(
                    "CAD conversion tool NOT available: '{}' (formats: {:?})",
                    tool.command,
                    tool.formats
                );
            }
        }

        self.converter = Some(Arc::clone(&converter));
        tracing::info!("CAD converter plugin initialized");
        Ok(converter)
    }

    /// Register hooks with the hook registry
    pub fn register_hooks(&self, registry: &mut HookRegistry) -> Result<(), AppError> {
        let converter = self
            .converter
            .as_ref()
            .ok_or_else(|| AppError::internal("CAD converter plugin not initialized"))?;

        registry.register(
            "after_upload",
            Arc::new(AfterUploadHook::new(
                Arc::clone(converter),
                self.output_dir.clone(),
            )),
        );

        tracing::info!("CAD converter hooks registered: after_upload");
        Ok(())
    }

    /// Get plugin info
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    /// Get the converter
    pub fn converter(&self) -> Option<&Arc<CadConverter>> {
        self.converter.as_ref()
    }
}

impl Default for CadConverterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook handler for after_upload: check if file is CAD and queue conversion
struct AfterUploadHook {
    /// The CAD converter
    converter: Arc<CadConverter>,
    /// Output directory for conversions
    output_dir: PathBuf,
}

impl AfterUploadHook {
    /// Create a new after_upload hook handler
    fn new(converter: Arc<CadConverter>, output_dir: PathBuf) -> Self {
        Self {
            converter,
            output_dir,
        }
    }
}

impl std::fmt::Debug for AfterUploadHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AfterUploadHook")
            .field("output_dir", &self.output_dir)
            .finish()
    }
}

#[async_trait]
impl HookHandler for AfterUploadHook {
    fn name(&self) -> &str {
        "cad_converter_after_upload"
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn execute(&self, ctx: &HookContext, payload: &HookPayload) -> HookResult {
        let file_name = match payload.get_str("file_name") {
            Some(name) => name.to_string(),
            None => {
                tracing::debug!("after_upload: no file_name in payload, skipping CAD check");
                return Ok(HookAction::Continue(None));
            }
        };

        if !self.converter.is_convertible(&file_name) {
            tracing::debug!(
                "File '{}' is not a CAD file, skipping conversion",
                file_name
            );
            return Ok(HookAction::Continue(None));
        }

        let file_id_str = payload.get_str("file_id").unwrap_or_default();
        let source_path_str = payload.get_str("storage_path").unwrap_or_default();

        let format = self.converter.detect_format(&file_name);
        let targets = self.converter.supported_targets(&file_name);

        tracing::info!(
            "CAD file detected: '{}' (format: {:?}), queuing conversion to {:?}",
            file_name,
            format,
            targets
        );

        let job_payload = serde_json::json!({
            "type": "cad_conversion",
            "file_id": file_id_str,
            "file_name": file_name,
            "source_path": source_path_str,
            "targets": targets,
            "output_dir": self.output_dir.to_string_lossy(),
            "format": format,
        });

        Ok(HookAction::Continue(Some(serde_json::json!({
            "queue_job": {
                "job_type": "cad_conversion",
                "queue": "conversion",
                "priority": "normal",
                "payload": job_payload,
            }
        }))))
    }
}
