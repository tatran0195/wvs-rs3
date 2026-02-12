//! FileHub plugin trait implementation and hook registration.
//!
//! Integrates the CAD conversion processor with the FileHub plugin system.
//! Uses the flat `HookPayload` data bag to extract file upload information
//! and signal conversion requirements via `HookResult` output data.

use std::sync::Arc;
use tracing::{debug, info, warn};

use filehub_core::error::AppError;
use filehub_plugin::{HookRegistry, prelude::*};

use crate::config::ConversionConfig;
use crate::metrics::MetricsSnapshot;
use crate::models::FileType;
use crate::processor::ConversionProcessor;

/// Plugin name used for registration, logging, and hook results.
const PLUGIN_NAME: &str = "cad-converter";

/// Plugin version from Cargo manifest.
const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Well-known payload data keys for file upload hooks.
///
/// These keys are expected to be set by the core file upload handler
/// before the `AfterUpload` hook fires.
#[allow(dead_code)]
mod payload_keys {
    /// File name of the uploaded file (String).
    pub const FILE_NAME: &str = "file_name";
    /// UUID of the file entity (String, parseable as UUID).
    pub const FILE_ID: &str = "file_id";
    /// UUID of the storage backend (String, parseable as UUID).
    pub const STORAGE_ID: &str = "storage_id";
    /// Storage path where the file was written (String).
    pub const STORAGE_PATH: &str = "storage_path";
    /// File size in bytes (i64).
    pub const FILE_SIZE: &str = "file_size";
    /// UUID of the user who uploaded (String, parseable as UUID).
    pub const UPLOADED_BY: &str = "uploaded_by";
    /// MIME type of the uploaded file (String).
    pub const MIME_TYPE: &str = "mime_type";
}

/// Well-known output keys set by this plugin in hook results.
///
/// The worker system reads these keys to determine if a conversion
/// job should be created.
mod output_keys {
    /// Whether CAD conversion is required (bool).
    pub const CONVERSION_REQUIRED: &str = "cad_conversion_required";
    /// Detected file type name (String, debug format of FileType).
    pub const FILE_TYPE: &str = "cad_file_type";
    /// Input path for the conversion (String).
    pub const INPUT_PATH: &str = "cad_input_path";
    /// File entity ID (String).
    pub const FILE_ID: &str = "cad_file_id";
    /// Storage ID (String).
    pub const STORAGE_ID: &str = "cad_storage_id";
    /// Whether this is a VTFx pass-through (no Jupiter needed) (bool).
    pub const IS_PASSTHROUGH: &str = "cad_is_vtfx_passthrough";
    /// Available conversion slots at time of detection (i64).
    pub const AVAILABLE_SLOTS: &str = "cad_available_slots";
    /// Whether the file type is a results/post format (bool).
    pub const IS_RESULTS: &str = "cad_is_results_format";
}

/// The CAD converter plugin for FileHub.
///
/// Wraps the `ConversionProcessor` and exposes it to the FileHub plugin
/// system via hooks. The `after_upload` hook inspects uploaded files and
/// signals conversion requirements to the worker system.
#[derive(Debug)]
pub struct CadConverterPlugin {
    /// Plugin configuration.
    config: ConversionConfig,
    /// The conversion processor (created on initialize).
    processor: Arc<tokio::sync::RwLock<Option<Arc<ConversionProcessor>>>>,
    /// Whether the plugin has been successfully initialized.
    initialized: Arc<tokio::sync::RwLock<bool>>,
}

impl CadConverterPlugin {
    /// Create a new plugin with default configuration.
    pub fn new() -> Self {
        Self {
            config: ConversionConfig::default(),
            processor: Arc::new(tokio::sync::RwLock::new(None)),
            initialized: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Create a new plugin with the given configuration.
    pub fn with_config(config: ConversionConfig) -> Self {
        Self {
            config,
            processor: Arc::new(tokio::sync::RwLock::new(None)),
            initialized: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }
}

impl CadConverterPlugin {
    /// Initialize the plugin: resolve Jupiter path and create the processor.
    pub async fn initialize(&self) -> Result<Arc<ConversionProcessor>, AppError> {
        if !self.config.enabled {
            info!(plugin = PLUGIN_NAME, "CAD converter plugin is disabled");
            return Err(AppError::internal("CAD converter plugin is disabled"));
        }

        info!(
            plugin = PLUGIN_NAME,
            version = PLUGIN_VERSION,
            "Initializing CAD converter plugin"
        );

        // Resolve Jupiter-Web installation (auto-discover if not configured)
        let mut config = self.config.clone();
        match config.resolve_jupiter_path() {
            Ok(path) => {
                info!(
                    jupiter_path = %path.display(),
                    summary = %config.jupiter_summary(),
                    "Jupiter-Web resolved"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Jupiter-Web not found — plugin will initialize but conversions will fail. \
                     Install Jupiter-Web or set jupiter_path in config."
                );
                // Don't fail initialization — allow the plugin to load so it can
                // report the error when a conversion is actually attempted
            }
        }

        info!(
            max_global = config.max_global_concurrency,
            max_io = config.max_io_concurrency,
            timeout_s = config.jupiter_timeout_seconds,
            max_retries = config.max_retries,
            "Concurrency and timeout settings"
        );

        let processor = ConversionProcessor::new(config.clone()).map_err(|e| {
            AppError::internal(format!("Failed to create conversion processor: {}", e))
        })?;

        info!(
            extensions = FileType::SUPPORTED_EXTENSIONS.len(),
            temp_root = %config.effective_temp_root().display(),
            jupiter_resolved = config.is_jupiter_resolved(),
            "CAD converter initialized"
        );

        let processor = Arc::new(processor);
        let mut proc_lock = self.processor.write().await;
        *proc_lock = Some(processor.clone());
        let mut init_lock = self.initialized.write().await;
        *init_lock = true;
        Ok(processor)
    }

    /// Shut down the plugin gracefully.
    pub async fn shutdown(&self) -> Result<(), AppError> {
        let proc_lock = self.processor.read().await;
        if let Some(proc) = &*proc_lock {
            let snap = proc.metrics_snapshot();
            info!(
                plugin = PLUGIN_NAME,
                started = snap.conversions_started,
                succeeded = snap.conversions_succeeded,
                failed = snap.conversions_failed,
                timed_out = snap.conversions_timed_out,
                "CAD converter shutting down — final metrics"
            );
        }
        Ok(())
    }

    /// Register hooks with the FileHub hook registry.
    pub async fn register_hooks(&self, registry: &HookRegistry) {
        if !self.config.enabled {
            return;
        }

        let proc_lock = self.processor.read().await;
        let processor = match &*proc_lock {
            Some(p) => Arc::clone(p),
            None => {
                warn!(
                    plugin = PLUGIN_NAME,
                    "Cannot register hooks: plugin not initialized"
                );
                return;
            }
        };

        registry
            .register(
                HookPoint::AfterUpload,
                SimpleHandlerAdapter::wrap(Arc::new(AfterUploadHandler {
                    processor: processor.clone(),
                })),
            )
            .await;

        registry
            .register(
                HookPoint::OnServerStart,
                SimpleHandlerAdapter::wrap(Arc::new(OnServerStartHandler {
                    processor: processor.clone(),
                })),
            )
            .await;

        registry
            .register(
                HookPoint::OnServerShutdown,
                SimpleHandlerAdapter::wrap(Arc::new(OnServerShutdownHandler { processor })),
            )
            .await;

        info!(
            plugin = PLUGIN_NAME,
            "Registered hooks: after_upload, on_server_start, on_server_shutdown"
        );
    }

    /// Check if a filename is a supported CAD/FEA file type.
    pub fn is_supported_file(filename: &str) -> bool {
        FileType::from_path_ref(std::path::Path::new(filename))
            .map(|ft| ft.is_processable())
            .unwrap_or(false)
    }

    /// Get the processor reference (if initialized).
    pub async fn processor(&self) -> Option<Arc<ConversionProcessor>> {
        self.processor.read().await.clone()
    }

    /// Get a metrics snapshot (if initialized).
    pub async fn metrics_snapshot(&self) -> Option<MetricsSnapshot> {
        self.processor
            .read()
            .await
            .as_ref()
            .map(|p| p.metrics_snapshot())
    }

    /// Whether the plugin is initialized.
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Get the configuration.
    pub fn config(&self) -> &ConversionConfig {
        &self.config
    }
}

#[async_trait]
impl Plugin for CadConverterPlugin {
    fn info(&self) -> PluginInfo {
        plugin_info!(
            id: PLUGIN_NAME,
            name: "CAD Converter",
            version: PLUGIN_VERSION,
            description: "TechnoStar Jupiter-based CAD/FEA converter plugin",
            author: "TechnoStar"
        )
    }

    async fn on_load(&self) -> Result<(), String> {
        info!(plugin = PLUGIN_NAME, "Plugin loaded");
        Ok(())
    }

    async fn on_start(&self) -> Result<(), String> {
        info!(plugin = PLUGIN_NAME, "Plugin started");
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), String> {
        info!(plugin = PLUGIN_NAME, "Plugin stopped");
        Ok(())
    }

    async fn on_unload(&self) -> Result<(), String> {
        info!(plugin = PLUGIN_NAME, "Plugin unloaded");
        Ok(())
    }

    fn registered_hooks(&self) -> Vec<HookPoint> {
        vec![
            HookPoint::AfterUpload,
            HookPoint::OnServerStart,
            HookPoint::OnServerShutdown,
        ]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// AfterUpload hook handler
// ---------------------------------------------------------------------------

/// Handles the `after_upload` hook point.
///
/// When a file is uploaded, this handler:
/// 1. Reads the file name from the payload data bag
/// 2. Checks if the extension maps to a supported CAD/FEA type
/// 3. If supported, returns a `HookResult` with output data signaling
///    the worker system to create a conversion job
/// 4. If not supported, returns a simple continue result
#[derive(Debug)]
struct AfterUploadHandler {
    /// Reference to the shared conversion processor.
    processor: Arc<ConversionProcessor>,
}

#[async_trait]
impl SimpleHookHandler for AfterUploadHandler {
    fn plugin_id(&self) -> &str {
        PLUGIN_NAME
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::AfterUpload
    }

    async fn handle(&self, payload: &HookPayload) -> HookResult {
        // Extract file name from the payload data bag
        let file_name = match payload.get_string(payload_keys::FILE_NAME) {
            Some(name) => name,
            None => {
                debug!(
                    hook = %payload.hook,
                    "after_upload payload missing '{}' key, skipping",
                    payload_keys::FILE_NAME
                );
                return HookResult::continue_execution(PLUGIN_NAME);
            }
        };

        // Determine file type from the name
        let file_type = match FileType::from_path_ref(std::path::Path::new(file_name)) {
            Some(ft) if ft.is_processable() => ft,
            _ => {
                debug!(
                    file = %file_name,
                    "File is not a supported CAD/FEA format, skipping"
                );
                return HookResult::continue_execution(PLUGIN_NAME);
            }
        };

        // Extract additional metadata from the payload
        let file_id = payload
            .get_string(payload_keys::FILE_ID)
            .unwrap_or("unknown");
        let storage_id = payload
            .get_string(payload_keys::STORAGE_ID)
            .unwrap_or("unknown");
        let storage_path = payload.get_string(payload_keys::STORAGE_PATH).unwrap_or("");

        info!(
            file = %file_name,
            file_type = ?file_type,
            file_id = %file_id,
            "CAD/FEA file detected, signaling for conversion"
        );

        // Build output data that the worker system will read
        let mut output = serde_json::Map::new();
        output.insert(
            output_keys::CONVERSION_REQUIRED.to_string(),
            serde_json::Value::Bool(true),
        );
        output.insert(
            output_keys::FILE_TYPE.to_string(),
            serde_json::Value::String(format!("{:?}", file_type)),
        );
        output.insert(
            output_keys::INPUT_PATH.to_string(),
            serde_json::Value::String(storage_path.to_string()),
        );
        output.insert(
            output_keys::FILE_ID.to_string(),
            serde_json::Value::String(file_id.to_string()),
        );
        output.insert(
            output_keys::STORAGE_ID.to_string(),
            serde_json::Value::String(storage_id.to_string()),
        );
        output.insert(
            output_keys::IS_PASSTHROUGH.to_string(),
            serde_json::Value::Bool(file_type.is_vtfx_format()),
        );
        output.insert(
            output_keys::IS_RESULTS.to_string(),
            serde_json::Value::Bool(file_type.is_results_format()),
        );
        output.insert(
            output_keys::AVAILABLE_SLOTS.to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                self.processor.available_global_slots(),
            )),
        );

        HookResult::continue_with_output(PLUGIN_NAME, serde_json::Value::Object(output))
    }
}

// ---------------------------------------------------------------------------
// OnServerStart hook handler
// ---------------------------------------------------------------------------

/// Handles the `on_server_start` hook point — logs readiness info.
#[derive(Debug)]
struct OnServerStartHandler {
    /// Reference to the shared conversion processor.
    processor: Arc<ConversionProcessor>,
}

#[async_trait]
impl SimpleHookHandler for OnServerStartHandler {
    fn plugin_id(&self) -> &str {
        PLUGIN_NAME
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::OnServerStart
    }

    async fn handle(&self, _payload: &HookPayload) -> HookResult {
        info!(
            plugin = PLUGIN_NAME,
            extensions = FileType::SUPPORTED_EXTENSIONS.len(),
            slots = self.processor.available_global_slots(),
            "CAD converter plugin ready"
        );
        HookResult::continue_execution(PLUGIN_NAME)
    }
}

// ---------------------------------------------------------------------------
// OnServerShutdown hook handler
// ---------------------------------------------------------------------------

/// Handles the `on_server_shutdown` hook point — logs final metrics.
#[derive(Debug)]
struct OnServerShutdownHandler {
    /// Reference to the shared conversion processor.
    processor: Arc<ConversionProcessor>,
}

#[async_trait]
impl SimpleHookHandler for OnServerShutdownHandler {
    fn plugin_id(&self) -> &str {
        PLUGIN_NAME
    }

    fn hook_point(&self) -> HookPoint {
        HookPoint::OnServerShutdown
    }

    async fn handle(&self, _payload: &HookPayload) -> HookResult {
        let snap = self.processor.metrics_snapshot();
        info!(
            plugin = PLUGIN_NAME,
            started = snap.conversions_started,
            succeeded = snap.conversions_succeeded,
            failed = snap.conversions_failed,
            timed_out = snap.conversions_timed_out,
            cancelled = snap.conversions_cancelled,
            vtfx_passthrough = snap.vtfx_passthrough_count,
            output_bytes = snap.total_output_bytes,
            "CAD converter final metrics"
        );
        HookResult::continue_execution(PLUGIN_NAME)
    }
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/// Plugin factory — creates a new instance with default configuration.
pub fn create_plugin() -> CadConverterPlugin {
    CadConverterPlugin::new()
}

/// Plugin factory with custom configuration.
pub fn create_plugin_with_config(config: ConversionConfig) -> CadConverterPlugin {
    CadConverterPlugin::with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use filehub_plugin::HookAction;

    #[test]
    fn test_is_supported_file() {
        // CAD formats
        assert!(CadConverterPlugin::is_supported_file("model.stp"));
        assert!(CadConverterPlugin::is_supported_file("model.step"));
        assert!(CadConverterPlugin::is_supported_file("drawing.dwg"));
        assert!(CadConverterPlugin::is_supported_file("part.x_t"));
        assert!(CadConverterPlugin::is_supported_file("part.sldprt"));

        // FEA formats
        assert!(CadConverterPlugin::is_supported_file("mesh.bdf"));
        assert!(CadConverterPlugin::is_supported_file("mesh.dat"));
        assert!(CadConverterPlugin::is_supported_file("results.op2"));

        // VTFx pass-through
        assert!(CadConverterPlugin::is_supported_file("vis.vtfx"));

        // Not supported
        assert!(!CadConverterPlugin::is_supported_file("document.pdf"));
        assert!(!CadConverterPlugin::is_supported_file("image.png"));
        assert!(!CadConverterPlugin::is_supported_file("readme.md"));
        assert!(!CadConverterPlugin::is_supported_file("archive.tar.gz"));
    }

    #[tokio::test]
    async fn test_plugin_default_state() {
        let plugin = CadConverterPlugin::new();
        assert!(!plugin.is_initialized().await);
        assert!(plugin.processor().await.is_none());
        assert!(plugin.metrics_snapshot().await.is_none());
        assert!(plugin.config().enabled);
    }

    #[test]
    fn test_plugin_disabled() {
        let config = ConversionConfig {
            enabled: false,
            ..Default::default()
        };
        let plugin = CadConverterPlugin::with_config(config);
        assert!(!plugin.config().enabled);
    }

    #[tokio::test]
    async fn test_plugin_initialize_enabled() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_plugin_init_test")),
            ..Default::default()
        };
        let plugin = CadConverterPlugin::with_config(config);
        plugin.initialize().await.expect("should initialize");
        assert!(plugin.is_initialized().await);
        assert!(plugin.processor().await.is_some());
        assert!(plugin.metrics_snapshot().await.is_some());
    }

    #[tokio::test]
    async fn test_plugin_initialize_disabled() {
        let config = ConversionConfig {
            enabled: false,
            ..Default::default()
        };
        let plugin = CadConverterPlugin::with_config(config);
        plugin.initialize().await.expect("should succeed");
        assert!(!plugin.is_initialized().await);
        assert!(plugin.processor().await.is_none());
    }

    #[tokio::test]
    async fn test_plugin_shutdown_with_metrics() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_shutdown_test_v2")),
            ..Default::default()
        };
        let plugin = CadConverterPlugin::with_config(config);
        plugin.initialize().await.expect("init");
        plugin.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn test_after_upload_handler_cad_file() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_hook_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = AfterUploadHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::AfterUpload)
            .with_data(
                payload_keys::FILE_NAME,
                serde_json::Value::String("model.stp".to_string()),
            )
            .with_data(
                payload_keys::FILE_ID,
                serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
            )
            .with_data(
                payload_keys::STORAGE_ID,
                serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
            )
            .with_data(
                payload_keys::STORAGE_PATH,
                serde_json::Value::String("/storage/local/model.stp".to_string()),
            );

        let result = handler.handle(&payload).await;

        // Should continue with output data
        assert_eq!(result.plugin_id, PLUGIN_NAME);
        let output = result.output.expect("should have output");
        let obj = output.as_object().expect("should be object");
        assert_eq!(
            obj.get(output_keys::CONVERSION_REQUIRED),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            obj.get(output_keys::IS_PASSTHROUGH),
            Some(&serde_json::Value::Bool(false))
        );
        assert!(obj.get(output_keys::FILE_TYPE).is_some());
    }

    #[tokio::test]
    async fn test_after_upload_handler_non_cad_file() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_hook_noncad_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = AfterUploadHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::AfterUpload).with_data(
            payload_keys::FILE_NAME,
            serde_json::Value::String("document.pdf".to_string()),
        );

        let result = handler.handle(&payload).await;

        assert_eq!(result.plugin_id, PLUGIN_NAME);
        assert!(result.output.is_none());
        assert!(matches!(result.action, HookAction::Continue));
    }

    #[tokio::test]
    async fn test_after_upload_handler_vtfx_passthrough() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_hook_vtfx_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = AfterUploadHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::AfterUpload).with_data(
            payload_keys::FILE_NAME,
            serde_json::Value::String("visualization.vtfx".to_string()),
        );

        let result = handler.handle(&payload).await;

        let output = result.output.expect("should have output");
        let obj = output.as_object().expect("object");
        assert_eq!(
            obj.get(output_keys::CONVERSION_REQUIRED),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            obj.get(output_keys::IS_PASSTHROUGH),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[tokio::test]
    async fn test_after_upload_handler_missing_filename() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_hook_noname_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = AfterUploadHandler {
            processor: Arc::new(processor),
        };

        // Payload with no file_name key
        let payload = HookPayload::new(HookPoint::AfterUpload);

        let result = handler.handle(&payload).await;

        assert!(result.output.is_none());
        assert!(matches!(result.action, HookAction::Continue));
    }

    #[tokio::test]
    async fn test_after_upload_handler_results_format() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_hook_results_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = AfterUploadHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::AfterUpload).with_data(
            payload_keys::FILE_NAME,
            serde_json::Value::String("analysis.op2".to_string()),
        );

        let result = handler.handle(&payload).await;

        let output = result.output.expect("should have output");
        let obj = output.as_object().expect("object");
        assert_eq!(
            obj.get(output_keys::IS_RESULTS),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            obj.get(output_keys::IS_PASSTHROUGH),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[tokio::test]
    async fn test_server_start_handler() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_start_handler_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = OnServerStartHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::OnServerStart);
        let result = handler.handle(&payload).await;

        assert_eq!(result.plugin_id, PLUGIN_NAME);
        assert!(matches!(result.action, HookAction::Continue));
    }

    #[tokio::test]
    async fn test_server_shutdown_handler() {
        let config = ConversionConfig {
            enabled: true,
            temp_root: Some(std::env::temp_dir().join("filehub_stop_handler_test")),
            ..Default::default()
        };
        let processor = ConversionProcessor::new(config).expect("create");
        let handler = OnServerShutdownHandler {
            processor: Arc::new(processor),
        };

        let payload = HookPayload::new(HookPoint::OnServerShutdown);
        let result = handler.handle(&payload).await;

        assert_eq!(result.plugin_id, PLUGIN_NAME);
        assert!(matches!(result.action, HookAction::Continue));
    }

    #[test]
    fn test_factory_functions() {
        let p1 = create_plugin();
        assert!(p1.config().enabled);

        let p2 = create_plugin_with_config(ConversionConfig {
            enabled: false,
            ..Default::default()
        });
        assert!(!p2.config().enabled);
    }
}
