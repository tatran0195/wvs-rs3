//! All hook point definitions with typed payloads.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Enumeration of all hook points in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    // ── Lifecycle ──
    /// Fired when the server starts.
    OnServerStart,
    /// Fired when the server is shutting down.
    OnServerShutdown,
    /// Fired when a background worker starts.
    OnWorkerStart,

    // ── Auth ──
    /// Fired before login credentials are validated. Can modify or halt.
    BeforeLogin,
    /// Fired after a successful login.
    AfterLogin,
    /// Fired before logout processing begins. Can modify or halt.
    BeforeLogout,
    /// Fired after logout completes.
    AfterLogout,
    /// Fired when a session expires due to timeout.
    OnSessionExpired,

    // ── Session ──
    /// Fired before an admin terminates a session. Can halt.
    BeforeSessionTerminate,
    /// Fired after a session is terminated.
    AfterSessionTerminate,
    /// Fired when a session becomes idle.
    OnSessionIdle,
    /// Fired before a bulk termination operation.
    BeforeBulkTerminate,
    /// Fired after a bulk termination completes.
    AfterBulkTerminate,

    // ── File ──
    /// Fired before a file upload is accepted. Can modify or halt.
    BeforeUpload,
    /// Fired after a file upload completes.
    AfterUpload,
    /// Fired before a file download is served. Can modify or halt.
    BeforeDownload,
    /// Fired after a file download completes.
    AfterDownload,
    /// Fired before a file is deleted. Can halt.
    BeforeDelete,
    /// Fired after a file is deleted.
    AfterDelete,
    /// Fired when a file is moved.
    OnFileMove,
    /// Fired when a file is copied.
    OnFileCopy,

    // ── Share ──
    /// Fired before a share is created. Can modify or halt.
    BeforeShare,
    /// Fired after a share is created.
    AfterShare,
    /// Fired when a shared resource is accessed.
    OnShareAccess,

    // ── Admin ──
    /// Fired when a new user is created.
    OnUserCreate,
    /// Fired when a user is deleted.
    OnUserDelete,
    /// Fired when a storage backend is added.
    OnStorageAdd,
    /// Fired when system configuration changes.
    OnConfigChange,

    // ── Realtime ──
    /// Fired when a WebSocket connection is established.
    OnWsConnect,
    /// Fired when a WebSocket connection is closed.
    OnWsDisconnect,
    /// Fired when a client subscribes to a channel.
    OnChannelSubscribe,
    /// Fired before a notification is sent. Can modify or halt.
    BeforeNotificationSend,
    /// Fired when a user's presence status changes.
    OnPresenceChange,

    // ── Broadcast ──
    /// Fired before an admin broadcast is sent. Can modify or halt.
    BeforeAdminBroadcast,
    /// Fired after an admin broadcast is delivered.
    AfterAdminBroadcast,
}

impl HookPoint {
    /// Returns the string name of this hook point.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnServerStart => "on_server_start",
            Self::OnServerShutdown => "on_server_shutdown",
            Self::OnWorkerStart => "on_worker_start",
            Self::BeforeLogin => "before_login",
            Self::AfterLogin => "after_login",
            Self::BeforeLogout => "before_logout",
            Self::AfterLogout => "after_logout",
            Self::OnSessionExpired => "on_session_expired",
            Self::BeforeSessionTerminate => "before_session_terminate",
            Self::AfterSessionTerminate => "after_session_terminate",
            Self::OnSessionIdle => "on_session_idle",
            Self::BeforeBulkTerminate => "before_bulk_terminate",
            Self::AfterBulkTerminate => "after_bulk_terminate",
            Self::BeforeUpload => "before_upload",
            Self::AfterUpload => "after_upload",
            Self::BeforeDownload => "before_download",
            Self::AfterDownload => "after_download",
            Self::BeforeDelete => "before_delete",
            Self::AfterDelete => "after_delete",
            Self::OnFileMove => "on_file_move",
            Self::OnFileCopy => "on_file_copy",
            Self::BeforeShare => "before_share",
            Self::AfterShare => "after_share",
            Self::OnShareAccess => "on_share_access",
            Self::OnUserCreate => "on_user_create",
            Self::OnUserDelete => "on_user_delete",
            Self::OnStorageAdd => "on_storage_add",
            Self::OnConfigChange => "on_config_change",
            Self::OnWsConnect => "on_ws_connect",
            Self::OnWsDisconnect => "on_ws_disconnect",
            Self::OnChannelSubscribe => "on_channel_subscribe",
            Self::BeforeNotificationSend => "before_notification_send",
            Self::OnPresenceChange => "on_presence_change",
            Self::BeforeAdminBroadcast => "before_admin_broadcast",
            Self::AfterAdminBroadcast => "after_admin_broadcast",
        }
    }

    /// Returns whether this is a "before" hook that supports halt semantics.
    pub fn is_before_hook(&self) -> bool {
        matches!(
            self,
            Self::BeforeLogin
                | Self::BeforeLogout
                | Self::BeforeSessionTerminate
                | Self::BeforeBulkTerminate
                | Self::BeforeUpload
                | Self::BeforeDownload
                | Self::BeforeDelete
                | Self::BeforeShare
                | Self::BeforeNotificationSend
                | Self::BeforeAdminBroadcast
        )
    }
}

impl std::fmt::Display for HookPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Payload passed to hook handlers — a flexible key-value map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPayload {
    /// The hook point being fired.
    pub hook: HookPoint,
    /// Arbitrary data keyed by string.
    pub data: HashMap<String, serde_json::Value>,
    /// The actor (user) who triggered this event.
    pub actor_id: Option<Uuid>,
    /// Timestamp of the event.
    pub timestamp: DateTime<Utc>,
}

impl HookPayload {
    /// Creates a new hook payload.
    pub fn new(hook: HookPoint) -> Self {
        Self {
            hook,
            data: HashMap::new(),
            actor_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Sets the actor ID.
    pub fn with_actor(mut self, actor_id: Uuid) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    /// Inserts a typed data value.
    pub fn with_data(mut self, key: &str, value: serde_json::Value) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }

    /// Inserts a string value.
    pub fn with_string(self, key: &str, value: &str) -> Self {
        self.with_data(key, serde_json::json!(value))
    }

    /// Inserts a UUID value.
    pub fn with_uuid(self, key: &str, value: Uuid) -> Self {
        self.with_data(key, serde_json::json!(value))
    }

    /// Inserts an integer value.
    pub fn with_int(self, key: &str, value: i64) -> Self {
        self.with_data(key, serde_json::json!(value))
    }

    /// Inserts a boolean value.
    pub fn with_bool(self, key: &str, value: bool) -> Self {
        self.with_data(key, serde_json::json!(value))
    }

    /// Gets a data value by key.
    pub fn get_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Gets a string data value.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Gets a UUID data value.
    pub fn get_uuid(&self, key: &str) -> Option<Uuid> {
        self.data
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }

    /// Gets an i64 data value.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(|v| v.as_i64())
    }

    /// Gets a bool data value.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }
}

/// Action returned by a hook handler telling the dispatcher what to do next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookAction {
    /// Continue to the next handler.
    Continue,
    /// Continue but with modified payload data.
    ContinueWith(HashMap<String, serde_json::Value>),
    /// Halt execution — no further handlers or the main operation will run.
    Halt {
        /// Reason for halting.
        reason: String,
    },
}

/// Result returned from a hook handler invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// The action the handler wants the dispatcher to take.
    pub action: HookAction,
    /// Optional output data from the handler.
    pub output: Option<serde_json::Value>,
    /// Plugin ID that produced this result.
    pub plugin_id: String,
}

impl HookResult {
    /// Creates a continue result.
    pub fn continue_execution(plugin_id: &str) -> Self {
        Self {
            action: HookAction::Continue,
            output: None,
            plugin_id: plugin_id.to_string(),
        }
    }

    /// Creates a continue-with-modification result.
    pub fn continue_with(
        plugin_id: &str,
        modifications: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            action: HookAction::ContinueWith(modifications),
            output: None,
            plugin_id: plugin_id.to_string(),
        }
    }

    /// Creates a halt result.
    pub fn halt(plugin_id: &str, reason: &str) -> Self {
        Self {
            action: HookAction::Halt {
                reason: reason.to_string(),
            },
            output: None,
            plugin_id: plugin_id.to_string(),
        }
    }

    /// Creates a continue result with output data.
    pub fn continue_with_output(plugin_id: &str, output: serde_json::Value) -> Self {
        Self {
            action: HookAction::Continue,
            output: Some(output),
            plugin_id: plugin_id.to_string(),
        }
    }
}
