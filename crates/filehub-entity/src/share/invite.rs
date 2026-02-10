//! Share invite for direct user sharing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::permission::acl::AclPermission;

/// An invitation to share a resource directly with a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInvite {
    /// The share ID.
    pub share_id: Uuid,
    /// The resource being shared.
    pub resource_id: Uuid,
    /// The user being invited.
    pub invitee_id: Uuid,
    /// The permission level being granted.
    pub permission: AclPermission,
    /// The user who sent the invitation.
    pub invited_by: Uuid,
    /// A personal message (optional).
    pub message: Option<String>,
}
