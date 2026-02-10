//! System-level domain events.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// System-level events (admin actions, lifecycle, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SystemEvent {
    /// The server started up.
    ServerStarted {
        /// Server version.
        version: String,
    },
    /// The server is shutting down.
    ServerShutdown {
        /// Reason for shutdown.
        reason: String,
    },
    /// A storage backend was added.
    StorageAdded {
        /// The storage ID.
        storage_id: Uuid,
        /// The provider type.
        provider_type: String,
        /// The storage name.
        name: String,
    },
    /// A storage backend was removed.
    StorageRemoved {
        /// The storage ID.
        storage_id: Uuid,
        /// The storage name.
        name: String,
    },
    /// An admin broadcast was sent.
    AdminBroadcast {
        /// The broadcast ID.
        broadcast_id: Uuid,
        /// The admin who sent it.
        admin_id: Uuid,
        /// The broadcast title.
        title: String,
        /// Severity level.
        severity: String,
    },
    /// A configuration change was made.
    ConfigChanged {
        /// What section changed.
        section: String,
        /// Details of the change.
        details: Value,
    },
    /// A plugin was loaded.
    PluginLoaded {
        /// Plugin identifier.
        plugin_id: String,
        /// Plugin version.
        version: String,
    },
    /// A plugin was unloaded.
    PluginUnloaded {
        /// Plugin identifier.
        plugin_id: String,
    },
    /// License pool status changed.
    LicensePoolChanged {
        /// Total seats.
        total_seats: u32,
        /// Available seats.
        available: u32,
        /// Whether drift was detected.
        drift_detected: bool,
    },
}
