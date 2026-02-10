//! Cache key builders for all FileHub cache entries.
//!
//! Centralising key construction prevents typos and makes it easy
//! to find every key the application uses.

use uuid::Uuid;

/// Prefix applied to all FileHub cache keys.
const PREFIX: &str = "filehub";

// ── User keys ──────────────────────────────────────────────

/// Cache key for a user entity by ID.
pub fn user_by_id(user_id: Uuid) -> String {
    format!("{PREFIX}:user:{user_id}")
}

/// Cache key for a user entity by username.
pub fn user_by_username(username: &str) -> String {
    format!("{PREFIX}:user:name:{}", username.to_lowercase())
}

// ── Session keys ───────────────────────────────────────────

/// Cache key for a session entity by ID.
pub fn session_by_id(session_id: Uuid) -> String {
    format!("{PREFIX}:session:{session_id}")
}

/// Cache key for the active session count of a user.
pub fn user_active_session_count(user_id: Uuid) -> String {
    format!("{PREFIX}:session:count:{user_id}")
}

/// Cache key for the JWT blocklist (revoked tokens).
pub fn jwt_blocklist(token_hash: &str) -> String {
    format!("{PREFIX}:jwt:blocked:{token_hash}")
}

// ── Permission keys ────────────────────────────────────────

/// Cache key for effective permission of a user on a resource.
pub fn effective_permission(resource_type: &str, resource_id: Uuid, user_id: Uuid) -> String {
    format!("{PREFIX}:perm:{resource_type}:{resource_id}:{user_id}")
}

/// Pattern to invalidate all permission cache entries for a resource.
pub fn permission_resource_pattern(resource_type: &str, resource_id: Uuid) -> String {
    format!("{PREFIX}:perm:{resource_type}:{resource_id}:*")
}

/// Pattern to invalidate all permission cache entries for a user.
pub fn permission_user_pattern(user_id: Uuid) -> String {
    format!("{PREFIX}:perm:*:*:{user_id}")
}

// ── File / Folder keys ─────────────────────────────────────

/// Cache key for a file entity by ID.
pub fn file_by_id(file_id: Uuid) -> String {
    format!("{PREFIX}:file:{file_id}")
}

/// Cache key for a folder entity by ID.
pub fn folder_by_id(folder_id: Uuid) -> String {
    format!("{PREFIX}:folder:{folder_id}")
}

/// Cache key for the folder tree of a storage.
pub fn folder_tree(storage_id: Uuid) -> String {
    format!("{PREFIX}:tree:{storage_id}")
}

/// Cache key for files in a folder listing.
pub fn folder_files(folder_id: Uuid, page: u64) -> String {
    format!("{PREFIX}:folder_files:{folder_id}:p{page}")
}

// ── Storage keys ───────────────────────────────────────────

/// Cache key for a storage entity by ID.
pub fn storage_by_id(storage_id: Uuid) -> String {
    format!("{PREFIX}:storage:{storage_id}")
}

/// Cache key for the list of all storages.
pub fn storage_list() -> String {
    format!("{PREFIX}:storages:all")
}

// ── Share keys ─────────────────────────────────────────────

/// Cache key for a share entity by token.
pub fn share_by_token(token: &str) -> String {
    format!("{PREFIX}:share:token:{token}")
}

/// Cache key for a share entity by ID.
pub fn share_by_id(share_id: Uuid) -> String {
    format!("{PREFIX}:share:{share_id}")
}

// ── License / Seat keys ────────────────────────────────────

/// Cache key for the license pool status.
pub fn license_pool_status() -> String {
    format!("{PREFIX}:license:pool")
}

/// Cache key for the seat allocation lock.
pub fn seat_allocation_lock() -> String {
    format!("{PREFIX}:seat:lock")
}

/// Cache key for checked-out seat count.
pub fn seat_checked_out() -> String {
    format!("{PREFIX}:seat:checked_out")
}

/// Cache key for total seat count.
pub fn seat_total() -> String {
    format!("{PREFIX}:seat:total")
}

/// Cache key for admin-reserved seat count.
pub fn seat_admin_reserved() -> String {
    format!("{PREFIX}:seat:admin_reserved")
}

// ── Presence keys ──────────────────────────────────────────

/// Cache key for user presence state.
pub fn presence(user_id: Uuid) -> String {
    format!("{PREFIX}:presence:{user_id}")
}

/// Cache key for the set of all online users.
pub fn online_users() -> String {
    format!("{PREFIX}:presence:online")
}

// ── Notification keys ──────────────────────────────────────

/// Cache key for unread notification count.
pub fn unread_notification_count(user_id: Uuid) -> String {
    format!("{PREFIX}:notif:unread:{user_id}")
}

/// Cache key for notification preferences.
pub fn notification_preferences(user_id: Uuid) -> String {
    format!("{PREFIX}:notif:prefs:{user_id}")
}

// ── Rate limiting keys ─────────────────────────────────────

/// Cache key for a rate limit bucket.
pub fn rate_limit(endpoint: &str, identifier: &str) -> String {
    format!("{PREFIX}:rate:{endpoint}:{identifier}")
}

// ── Dedup keys ─────────────────────────────────────────────

/// Cache key for notification deduplication.
pub fn notification_dedup(user_id: Uuid, event_type: &str, resource_id: Uuid) -> String {
    format!("{PREFIX}:dedup:{user_id}:{event_type}:{resource_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_key() {
        let id = Uuid::nil();
        assert_eq!(
            user_by_id(id),
            "filehub:user:00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn test_permission_key() {
        let rid = Uuid::nil();
        let uid = Uuid::nil();
        assert_eq!(
            effective_permission("folder", rid, uid),
            "filehub:perm:folder:00000000-0000-0000-0000-000000000000:00000000-0000-0000-0000-000000000000"
        );
    }
}
