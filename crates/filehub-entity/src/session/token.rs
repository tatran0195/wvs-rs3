//! Token value types for JWT access and refresh tokens.

use serde::{Deserialize, Serialize};

/// An issued JWT access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// The raw JWT string.
    pub token: String,
    /// TTL in seconds.
    pub expires_in: u64,
}

/// An issued JWT refresh token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    /// The raw JWT string.
    pub token: String,
    /// TTL in seconds.
    pub expires_in: u64,
}

/// A pair of access and refresh tokens returned on login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// The access token.
    pub access_token: AccessToken,
    /// The refresh token.
    pub refresh_token: RefreshToken,
}
