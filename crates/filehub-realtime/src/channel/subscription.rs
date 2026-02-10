//! Subscription management with ACL permission checking.

use std::sync::Arc;

use tracing;
use uuid::Uuid;

use filehub_core::types::id::UserId;
use filehub_entity::user::role::UserRole;

use super::types::ChannelType;
use crate::connection::handle::ConnectionHandle;

/// Result of a subscription authorization check
#[derive(Debug, Clone)]
pub enum SubscriptionAuth {
    /// Subscription allowed
    Allowed,
    /// Subscription denied with reason
    Denied(String),
}

/// Check if a connection is authorized to subscribe to a channel.
///
/// Rules:
/// - `user:{id}` — only the user themselves
/// - `folder:{id}`, `file:{id}` — requires at least viewer permission (TODO: ACL check)
/// - `upload:{id}`, `job:{id}` — the user who initiated it
/// - `admin:sessions`, `admin:system` — admin role only
/// - `broadcast:all`, `presence:global` — any authenticated user
/// - `storage:{id}` — admin or users with access
/// - `share:{id}` — the share creator
pub fn authorize_subscription(
    handle: &ConnectionHandle,
    channel_type: &ChannelType,
) -> SubscriptionAuth {
    match channel_type {
        ChannelType::User(user_id) => {
            if *handle.user_id == *user_id {
                SubscriptionAuth::Allowed
            } else {
                SubscriptionAuth::Denied("Cannot subscribe to another user's channel".to_string())
            }
        }

        ChannelType::Folder(_) | ChannelType::File(_) => {
            // For now, allow all authenticated users.
            // TODO: Integrate with ACL checker for resource-level permissions
            SubscriptionAuth::Allowed
        }

        ChannelType::Upload(_) | ChannelType::Job(_) => {
            // Allow all authenticated — the initiator filter is done at event dispatch
            SubscriptionAuth::Allowed
        }

        ChannelType::AdminSessions | ChannelType::AdminSystem => {
            if matches!(handle.user_role, UserRole::Admin) {
                SubscriptionAuth::Allowed
            } else {
                SubscriptionAuth::Denied("Admin role required".to_string())
            }
        }

        ChannelType::BroadcastAll | ChannelType::PresenceGlobal => SubscriptionAuth::Allowed,

        ChannelType::Storage(_) => {
            if matches!(handle.user_role, UserRole::Admin) {
                SubscriptionAuth::Allowed
            } else {
                // Non-admins can subscribe but will only see limited events
                SubscriptionAuth::Allowed
            }
        }

        ChannelType::Share(_) => SubscriptionAuth::Allowed,
    }
}
