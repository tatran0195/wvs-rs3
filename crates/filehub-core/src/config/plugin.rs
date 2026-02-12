//! Plugin system configuration.

use serde::{Deserialize, Serialize};

/// Plugin system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Directory containing plugin shared libraries.
    #[serde(default = "default_plugin_directory")]
    pub directory: String,
    /// Whether to automatically load plugins on startup.
    #[serde(default = "default_true")]
    pub auto_load: bool,
}

fn default_plugin_directory() -> String {
    "./plugins".to_string()
}

fn default_true() -> bool {
    true
}
