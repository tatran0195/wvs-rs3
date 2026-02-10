//! Complete inbound and outbound message type definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use filehub_core::types::id::{FileId, FolderId, SessionId, UserId};

/// Messages received FROM clients via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMessage {
    /// Subscribe to a channel
    Subscribe {
        /// Channel name (e.g., "folder:{uuid}")
        channel: String,
    },

    /// Unsubscribe from a channel
    Unsubscribe {
        /// Channel name
        channel: String,
    },

    /// Acknowledge receipt of a message
    Ack {
        /// Message ID being acknowledged
        message_id: String,
    },

    /// Mark a notification as read
    MarkRead {
        /// Notification ID
        notification_id: Uuid,
    },

    /// Mark all notifications as read
    MarkAllRead,

    /// Update user presence status
    PresenceUpdate {
        /// New status: "active", "idle", "away", "dnd"
        status: String,
    },

    /// Pong response to server ping
    Pong {
        /// Timestamp from the original ping
        timestamp: DateTime<Utc>,
    },

    /// Typing indicator (for future chat/comments)
    Typing {
        /// Resource being typed in
        channel: String,
    },

    /// Heartbeat from client
    Heartbeat,
}

/// Messages sent TO clients via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMessage {
    // ── Connection lifecycle ─────────────────────────────────
    /// Connection established successfully
    Connected {
        /// Connection ID assigned by server
        connection_id: Uuid,
        /// Server timestamp
        server_time: DateTime<Utc>,
    },

    /// Server ping (client should respond with Pong)
    Ping {
        /// Server timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Subscription responses ───────────────────────────────
    /// Subscription confirmed
    Subscribed {
        /// Channel name
        channel: String,
    },

    /// Unsubscription confirmed
    Unsubscribed {
        /// Channel name
        channel: String,
    },

    /// Subscription denied
    SubscriptionDenied {
        /// Channel name
        channel: String,
        /// Reason for denial
        reason: String,
    },

    // ── File events ──────────────────────────────────────────
    /// A file was created/uploaded
    FileCreated {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Folder it was created in
        folder_id: Uuid,
        /// Who created it
        actor_id: Uuid,
        /// Actor username
        actor_name: String,
        /// File size
        size_bytes: i64,
        /// MIME type
        mime_type: Option<String>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was updated (metadata or content)
    FileUpdated {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// What changed
        changes: Vec<String>,
        /// Who updated it
        actor_id: Uuid,
        /// Actor username
        actor_name: String,
        /// New version number (if content changed)
        version: Option<i32>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was deleted
    FileDeleted {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Folder it was in
        folder_id: Uuid,
        /// Who deleted it
        actor_id: Uuid,
        /// Actor username
        actor_name: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was moved
    FileMoved {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Source folder
        from_folder_id: Uuid,
        /// Destination folder
        to_folder_id: Uuid,
        /// Who moved it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was copied
    FileCopied {
        /// Original file ID
        source_file_id: Uuid,
        /// New copy file ID
        new_file_id: Uuid,
        /// File name
        file_name: String,
        /// Destination folder
        to_folder_id: Uuid,
        /// Who copied it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was locked
    FileLocked {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Who locked it
        locked_by: Uuid,
        /// Locker username
        locked_by_name: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A file was unlocked
    FileUnlocked {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Who unlocked it
        unlocked_by: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A new file version was created
    FileVersionCreated {
        /// File ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// New version number
        version: i32,
        /// Who created the version
        actor_id: Uuid,
        /// Comment on the version
        comment: Option<String>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Folder events ────────────────────────────────────────
    /// A folder was created
    FolderCreated {
        /// Folder ID
        folder_id: Uuid,
        /// Folder name
        folder_name: String,
        /// Parent folder ID (None if root)
        parent_id: Option<Uuid>,
        /// Who created it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A folder was renamed/updated
    FolderUpdated {
        /// Folder ID
        folder_id: Uuid,
        /// New name
        folder_name: String,
        /// Who updated it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A folder was deleted
    FolderDeleted {
        /// Folder ID
        folder_id: Uuid,
        /// Folder name
        folder_name: String,
        /// Parent folder ID
        parent_id: Option<Uuid>,
        /// Who deleted it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A folder was moved
    FolderMoved {
        /// Folder ID
        folder_id: Uuid,
        /// Folder name
        folder_name: String,
        /// Old parent
        from_parent_id: Option<Uuid>,
        /// New parent
        to_parent_id: Option<Uuid>,
        /// Who moved it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Upload progress ──────────────────────────────────────
    /// Upload chunk received
    UploadProgress {
        /// Upload session ID
        upload_id: Uuid,
        /// File name being uploaded
        file_name: String,
        /// Chunk number just received
        chunk_number: i32,
        /// Total chunks
        total_chunks: i32,
        /// Bytes uploaded so far
        bytes_uploaded: i64,
        /// Total file size
        total_bytes: i64,
        /// Progress percentage (0-100)
        percent: f64,
    },

    /// Upload completed
    UploadCompleted {
        /// Upload session ID
        upload_id: Uuid,
        /// Created file ID
        file_id: Uuid,
        /// File name
        file_name: String,
        /// Final file size
        size_bytes: i64,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// Upload failed
    UploadFailed {
        /// Upload session ID
        upload_id: Uuid,
        /// File name
        file_name: String,
        /// Error message
        error: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Share events ─────────────────────────────────────────
    /// A share was created
    ShareCreated {
        /// Share ID
        share_id: Uuid,
        /// Resource type ("file" or "folder")
        resource_type: String,
        /// Resource ID
        resource_id: Uuid,
        /// Resource name
        resource_name: String,
        /// Who created the share
        actor_id: Uuid,
        /// Share type ("public_link", "private_link", "user_share")
        share_type: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A shared resource was accessed
    ShareAccessed {
        /// Share ID
        share_id: Uuid,
        /// Resource name
        resource_name: String,
        /// Accessor info (IP or user)
        accessor: String,
        /// Download count
        download_count: i32,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A share was revoked
    ShareRevoked {
        /// Share ID
        share_id: Uuid,
        /// Resource name
        resource_name: String,
        /// Who revoked it
        actor_id: Uuid,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Notification ─────────────────────────────────────────
    /// A notification for the user
    Notification {
        /// Notification ID
        id: Uuid,
        /// Category
        category: String,
        /// Event type
        event_type: String,
        /// Title
        title: String,
        /// Message body
        message: String,
        /// Additional payload
        payload: Option<serde_json::Value>,
        /// Priority
        priority: String,
        /// Who caused this notification
        actor_id: Option<Uuid>,
        /// Actor name
        actor_name: Option<String>,
        /// Resource type
        resource_type: Option<String>,
        /// Resource ID
        resource_id: Option<Uuid>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// Unread notification count update
    UnreadCount {
        /// Number of unread notifications
        count: i64,
    },

    // ── Presence events ──────────────────────────────────────
    /// A user came online
    UserOnline {
        /// User ID
        user_id: Uuid,
        /// Username
        username: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A user went offline
    UserOffline {
        /// User ID
        user_id: Uuid,
        /// Username
        username: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A user changed their presence status
    PresenceChanged {
        /// User ID
        user_id: Uuid,
        /// Username
        username: String,
        /// New status
        status: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Session events (admin) ───────────────────────────────
    /// A new session was created (admin channel)
    SessionCreated {
        /// Session ID
        session_id: SessionId,
        /// User ID
        user_id: Uuid,
        /// Username
        username: String,
        /// IP address
        ip_address: String,
        /// User role
        role: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// A session was terminated
    SessionTerminated {
        /// Session ID
        session_id: SessionId,
        /// Reason for termination
        reason: String,
        /// Timestamp
        terminated_at: DateTime<Utc>,
    },

    /// Session count updated (admin channel)
    SessionCountUpdated {
        /// Active session count
        active_sessions: i32,
        /// Total seat capacity
        total_seats: i32,
        /// Available seats
        available_seats: i32,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Job events ───────────────────────────────────────────
    /// A job started
    JobStarted {
        /// Job ID
        job_id: Uuid,
        /// Job type
        job_type: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// Job progress update
    JobProgress {
        /// Job ID
        job_id: Uuid,
        /// Job type
        job_type: String,
        /// Progress percentage (0-100)
        percent: f64,
        /// Status message
        message: String,
    },

    /// Job completed
    JobCompleted {
        /// Job ID
        job_id: Uuid,
        /// Job type
        job_type: String,
        /// Result data
        result: Option<serde_json::Value>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// Job failed
    JobFailed {
        /// Job ID
        job_id: Uuid,
        /// Job type
        job_type: String,
        /// Error message
        error: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Admin broadcast ──────────────────────────────────────
    /// Admin broadcast message
    AdminBroadcast {
        /// Broadcast ID
        broadcast_id: Uuid,
        /// Title
        title: String,
        /// Message
        message: String,
        /// Severity: "info", "warning", "critical"
        severity: String,
        /// Whether it should persist on screen
        persistent: bool,
        /// Optional action
        action_type: Option<String>,
        /// Action payload
        action_payload: Option<serde_json::Value>,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Storage events ───────────────────────────────────────
    /// Storage status changed
    StorageStatusChanged {
        /// Storage ID
        storage_id: Uuid,
        /// Storage name
        storage_name: String,
        /// New status
        status: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    /// Storage quota warning
    StorageQuotaWarning {
        /// Storage ID
        storage_id: Uuid,
        /// Storage name
        storage_name: String,
        /// Used bytes
        used_bytes: i64,
        /// Quota bytes
        quota_bytes: i64,
        /// Utilization percentage
        utilization_percent: f64,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── License/Pool events ──────────────────────────────────
    /// Pool status update (admin channel)
    PoolStatusUpdated {
        /// Total seats
        total_seats: i32,
        /// Checked out
        checked_out: i32,
        /// Available
        available: i32,
        /// Drift detected
        drift_detected: bool,
        /// Timestamp
        timestamp: DateTime<Utc>,
    },

    // ── Errors ───────────────────────────────────────────────
    /// Error message
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
        /// Related request (if any)
        request_id: Option<String>,
    },
}
