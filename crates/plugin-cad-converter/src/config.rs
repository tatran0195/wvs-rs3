//! Configuration for the CAD conversion subsystem.
//!
//! Supports auto-discovery of Jupiter-Web installations via the Windows
//! registry Inno Setup GUID, falling back to common paths and PATH.

use crate::jupiter::{DiscoveryMethod, JupiterDiscovery, JupiterInstallation};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};
use validator::Validate;

/// Configuration for the TechnoStar Jupiter-based CAD converter.
///
/// If `jupiter_path` is not explicitly set (or is empty), the plugin will
/// attempt to auto-discover the Jupiter-Web installation by querying:
/// 1. Windows registry (Inno Setup GUID `{700798F8-7038-4887-BCC5-37278433D213}`)
/// 2. Common installation directories
/// 3. System PATH
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(default)]
pub struct ConversionConfig {
    /// Path to the Jupiter-Web launcher (Start_It.bat).
    ///
    /// If empty or not set, auto-discovery will be attempted.
    /// Set to an explicit path to skip auto-discovery.
    #[serde(default)]
    pub jupiter_path: PathBuf,

    /// Global limit for concurrent heavy Jupiter instances (CPU/RAM bound).
    #[serde(default = "default_max_global_concurrency")]
    #[validate(range(min = 1, max = 4))]
    pub max_global_concurrency: usize,

    /// Concurrency limit for lightweight IO operations (file copy/moves).
    #[serde(default = "default_max_io_concurrency")]
    #[validate(range(min = 1, max = 16))]
    pub max_io_concurrency: usize,

    /// Root directory for temporary conversion working files.
    #[serde(default)]
    pub temp_root: Option<PathBuf>,

    /// Whether the plugin is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Timeout in seconds for a single Jupiter process invocation.
    #[serde(default = "default_jupiter_timeout_seconds")]
    #[validate(range(min = 30, max = 7200))]
    pub jupiter_timeout_seconds: u64,

    /// Maximum retry attempts for transient Jupiter failures.
    #[serde(default = "default_max_retries")]
    #[validate(range(min = 0, max = 5))]
    pub max_retries: u32,

    /// Delay in seconds between retry attempts.
    #[serde(default = "default_retry_delay_seconds")]
    pub retry_delay_seconds: u64,

    /// Minimum output file size (bytes) to consider conversion successful.
    #[serde(default = "default_min_output_bytes")]
    pub min_output_bytes: u64,

    /// Whether to capture Jupiter stdout/stderr for diagnostics.
    #[serde(default = "default_capture_output")]
    pub capture_output: bool,

    /// Cached discovery result (not serialized, populated at runtime).
    #[serde(skip)]
    pub discovered_installation: Option<JupiterInstallation>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            jupiter_path: PathBuf::new(), // Empty = auto-discover
            max_global_concurrency: default_max_global_concurrency(),
            max_io_concurrency: default_max_io_concurrency(),
            temp_root: None,
            enabled: default_enabled(),
            jupiter_timeout_seconds: default_jupiter_timeout_seconds(),
            max_retries: default_max_retries(),
            retry_delay_seconds: default_retry_delay_seconds(),
            min_output_bytes: default_min_output_bytes(),
            capture_output: default_capture_output(),
            discovered_installation: None,
        }
    }
}

fn default_max_global_concurrency() -> usize {
    4
}

fn default_max_io_concurrency() -> usize {
    16
}

fn default_enabled() -> bool {
    true
}

fn default_jupiter_timeout_seconds() -> u64 {
    600
}

fn default_max_retries() -> u32 {
    1
}

fn default_retry_delay_seconds() -> u64 {
    5
}

fn default_min_output_bytes() -> u64 {
    100
}

fn default_capture_output() -> bool {
    true
}

impl ConversionConfig {
    /// Resolve the effective temp root directory.
    pub fn effective_temp_root(&self) -> PathBuf {
        self.temp_root
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("TechnoStar"))
    }

    /// Resolve the effective Jupiter launcher path.
    ///
    /// If `jupiter_path` is explicitly configured and non-empty, uses that.
    /// Otherwise, attempts auto-discovery via registry, common paths, and PATH.
    ///
    /// This method should be called once during plugin initialization.
    /// The result is cached in `discovered_installation`.
    pub fn resolve_jupiter_path(&mut self) -> Result<PathBuf, crate::error::ConversionError> {
        // If explicitly configured and non-empty, validate and use it
        if !self.jupiter_path.as_os_str().is_empty() {
            info!(
                path = %self.jupiter_path.display(),
                "Using explicitly configured Jupiter path"
            );

            match JupiterDiscovery::from_explicit_path(&self.jupiter_path) {
                Ok(installation) => {
                    let path = installation.launcher_path.clone();
                    self.discovered_installation = Some(installation);
                    return Ok(path);
                }
                Err(e) => {
                    warn!(
                        configured_path = %self.jupiter_path.display(),
                        error = %e,
                        "Configured Jupiter path is invalid, attempting auto-discovery"
                    );
                }
            }
        }

        // Auto-discover
        info!("Jupiter path not configured, attempting auto-discovery...");

        match JupiterDiscovery::discover() {
            Ok(installation) => {
                info!(
                    path = %installation.launcher_path.display(),
                    method = ?installation.discovery_method,
                    version = ?installation.display_version,
                    "Auto-discovered Jupiter-Web installation"
                );

                let path = installation.launcher_path.clone();
                self.jupiter_path = path.clone();
                self.discovered_installation = Some(installation);
                Ok(path)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    guid = %JupiterDiscovery::inno_setup_guid(),
                    "Jupiter-Web auto-discovery failed"
                );
                Err(crate::error::ConversionError::JupiterNotFound {
                    path: self.jupiter_path.clone(),
                })
            }
        }
    }

    /// Check if the Jupiter path has been resolved (either configured or discovered).
    pub fn is_jupiter_resolved(&self) -> bool {
        !self.jupiter_path.as_os_str().is_empty() && self.jupiter_path.exists()
    }

    /// Get discovery information (if auto-discovery was used).
    pub fn discovery_info(&self) -> Option<&JupiterInstallation> {
        self.discovered_installation.as_ref()
    }

    /// Get a human-readable summary of the Jupiter configuration.
    pub fn jupiter_summary(&self) -> String {
        match &self.discovered_installation {
            Some(inst) => {
                let name = inst.display_name.as_deref().unwrap_or("Jupiter-Web");
                let version = inst.display_version.as_deref().unwrap_or("unknown");
                let method = match inst.discovery_method {
                    DiscoveryMethod::WindowsRegistry => "registry",
                    DiscoveryMethod::CommonPath => "common path",
                    DiscoveryMethod::SystemPath => "system PATH",
                    DiscoveryMethod::ExplicitConfig => "explicit config",
                };
                format!(
                    "{} v{} at {} (found via {})",
                    name,
                    version,
                    inst.launcher_path.display(),
                    method
                )
            }
            None => {
                if self.jupiter_path.as_os_str().is_empty() {
                    "Not configured, auto-discovery not yet attempted".to_string()
                } else {
                    format!(
                        "Configured: {} (not validated)",
                        self.jupiter_path.display()
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_empty_jupiter_path() {
        let config = ConversionConfig::default();
        // Default jupiter_path should be empty (trigger auto-discovery)
        assert!(config.jupiter_path.as_os_str().is_empty());
        assert!(config.enabled);
        assert_eq!(config.max_global_concurrency, 4);
        assert_eq!(config.max_io_concurrency, 16);
        assert_eq!(config.jupiter_timeout_seconds, 600);
    }

    #[test]
    fn test_is_jupiter_resolved_empty() {
        let config = ConversionConfig::default();
        assert!(!config.is_jupiter_resolved());
    }

    #[test]
    fn test_is_jupiter_resolved_nonexistent() {
        let config = ConversionConfig {
            jupiter_path: PathBuf::from("/nonexistent/Start_It.bat"),
            ..Default::default()
        };
        assert!(!config.is_jupiter_resolved());
    }

    #[test]
    fn test_is_jupiter_resolved_real() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "echo").expect("write");

        let config = ConversionConfig {
            jupiter_path: launcher,
            ..Default::default()
        };
        assert!(config.is_jupiter_resolved());
    }

    #[test]
    fn test_resolve_explicit_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let launcher = temp.path().join("Start_It.bat");
        std::fs::write(&launcher, "@echo off").expect("write");

        let mut config = ConversionConfig {
            jupiter_path: launcher.clone(),
            ..Default::default()
        };

        let resolved = config.resolve_jupiter_path().expect("should resolve");
        assert_eq!(resolved, launcher);
        assert!(config.discovered_installation.is_some());

        let inst = config
            .discovered_installation
            .as_ref()
            .expect("has installation");
        assert_eq!(inst.discovery_method, DiscoveryMethod::ExplicitConfig);
    }

    #[test]
    fn test_jupiter_summary_not_configured() {
        let config = ConversionConfig::default();
        let summary = config.jupiter_summary();
        assert!(summary.contains("auto-discovery"));
    }

    #[test]
    fn test_jupiter_summary_configured() {
        let config = ConversionConfig {
            jupiter_path: PathBuf::from("C:/some/path/Start_It.bat"),
            ..Default::default()
        };
        let summary = config.jupiter_summary();
        assert!(summary.contains("Start_It.bat"));
    }

    #[test]
    fn test_jupiter_summary_discovered() {
        let config = ConversionConfig {
            jupiter_path: PathBuf::from("C:/Test/Start_It.bat"),
            discovered_installation: Some(JupiterInstallation {
                launcher_path: PathBuf::from("C:/Test/Start_It.bat"),
                install_dir: PathBuf::from("C:/Test"),
                display_name: Some("Jupiter-Web 5.0".to_string()),
                display_version: Some("5.0.2".to_string()),
                publisher: Some("TechnoStar".to_string()),
                discovery_method: DiscoveryMethod::WindowsRegistry,
            }),
            ..Default::default()
        };
        let summary = config.jupiter_summary();
        assert!(summary.contains("Jupiter-Web 5.0"));
        assert!(summary.contains("v5.0.2"));
        assert!(summary.contains("registry"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = ConversionConfig {
            jupiter_path: PathBuf::from("C:/Test/Start_It.bat"),
            max_global_concurrency: 2,
            max_io_concurrency: 8,
            jupiter_timeout_seconds: 300,
            max_retries: 3,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).expect("serialize");
        let deser: ConversionConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deser.max_global_concurrency, 2);
        assert_eq!(deser.max_io_concurrency, 8);
        assert_eq!(deser.jupiter_timeout_seconds, 300);
        assert_eq!(deser.max_retries, 3);
        // discovered_installation should be None after deserialization (#[serde(skip)])
        assert!(deser.discovered_installation.is_none());
    }

    #[test]
    fn test_toml_deserialization_empty() {
        // Simulate empty config section — all defaults should apply
        let toml_str = "[cad_converter]\nenabled = true\n";
        let config: ConversionConfig =
            toml::from_str(toml_str.trim_start_matches("[cad_converter]\n")).expect("parse toml");
        assert!(config.enabled);
        assert!(config.jupiter_path.as_os_str().is_empty());
    }
}
