//! JWT claims structure used in access and refresh tokens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use filehub_entity::user::UserRole;

/// JWT claims payload embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user ID.
    pub sub: Uuid,
    /// Session ID this token belongs to.
    pub sid: Uuid,
    /// User role at the time of token issuance.
    pub role: UserRole,
    /// Username for convenience.
    pub username: String,
    /// Issued-at timestamp (seconds since epoch).
    pub iat: i64,
    /// Expiration timestamp (seconds since epoch).
    pub exp: i64,
    /// JWT ID for blocklist tracking.
    pub jti: Uuid,
    /// Token type: "access" or "refresh".
    pub token_type: TokenType,
}

/// Distinguishes access tokens from refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// Short-lived access token for API requests.
    Access,
    /// Long-lived refresh token for obtaining new access tokens.
    Refresh,
}

impl Claims {
    /// Returns the user ID from the subject claim.
    pub fn user_id(&self) -> Uuid {
        self.sub
    }

    /// Returns the session ID.
    pub fn session_id(&self) -> Uuid {
        self.sid
    }

    /// Returns the expiration as a `DateTime<Utc>`.
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(|| Utc::now())
    }

    /// Checks whether this token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.exp
    }

    /// Returns the remaining TTL in seconds (0 if expired).
    pub fn remaining_ttl_seconds(&self) -> u64 {
        let remaining = self.exp - Utc::now().timestamp();
        if remaining > 0 { remaining as u64 } else { 0 }
    }
}
