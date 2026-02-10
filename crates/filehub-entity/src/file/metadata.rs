//! File metadata value object.

use serde::{Deserialize, Serialize};

/// Extended metadata stored as JSON alongside a file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Arbitrary tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Key-value custom properties.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl FileMetadata {
    /// Create empty metadata.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Convert to a `serde_json::Value`.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Parse from a `serde_json::Value`.
    pub fn from_json_value(value: &serde_json::Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}
