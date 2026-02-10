//! JWT token creation with configurable signing and TTL.

use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

use filehub_core::config::AuthConfig;
use filehub_core::error::AppError;
use filehub_entity::user::UserRole;

use super::claims::{Claims, TokenType};

/// Creates signed JWT access and refresh tokens.
#[derive(Debug, Clone)]
pub struct JwtEncoder {
    /// HMAC secret key for signing.
    encoding_key: EncodingKey,
    /// Access token TTL in minutes.
    access_ttl_minutes: i64,
    /// Refresh token TTL in hours.
    refresh_ttl_hours: i64,
}

/// Result of a successful token pair generation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenPair {
    /// Short-lived access token.
    pub access_token: String,
    /// Long-lived refresh token.
    pub refresh_token: String,
    /// Access token expiration timestamp.
    pub access_expires_at: chrono::DateTime<Utc>,
    /// Refresh token expiration timestamp.
    pub refresh_expires_at: chrono::DateTime<Utc>,
}

impl JwtEncoder {
    /// Creates a new encoder from auth configuration.
    pub fn new(config: &AuthConfig) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(config.jwt_secret.as_bytes()),
            access_ttl_minutes: config.jwt_access_ttl_minutes as i64,
            refresh_ttl_hours: config.jwt_refresh_ttl_hours as i64,
        }
    }

    /// Generates a new access + refresh token pair for the given user and session.
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        role: &UserRole,
        username: &str,
    ) -> Result<TokenPair, AppError> {
        let now = Utc::now();
        let access_exp = now + chrono::Duration::minutes(self.access_ttl_minutes);
        let refresh_exp = now + chrono::Duration::hours(self.refresh_ttl_hours);

        let access_claims = Claims {
            sub: user_id,
            sid: session_id,
            role: role.clone(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
            jti: Uuid::new_v4(),
            token_type: TokenType::Access,
        };

        let refresh_claims = Claims {
            sub: user_id,
            sid: session_id,
            role: role.clone(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
            jti: Uuid::new_v4(),
            token_type: TokenType::Refresh,
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode access token: {e}")))?;

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode refresh token: {e}")))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_expires_at: access_exp,
            refresh_expires_at: refresh_exp,
        })
    }

    /// Generates a standalone access token (e.g., after refresh).
    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        role: &UserRole,
        username: &str,
    ) -> Result<(String, chrono::DateTime<Utc>), AppError> {
        let now = Utc::now();
        let exp = now + chrono::Duration::minutes(self.access_ttl_minutes);

        let claims = Claims {
            sub: user_id,
            sid: session_id,
            role: role.clone(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: Uuid::new_v4(),
            token_type: TokenType::Access,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode access token: {e}")))?;

        Ok((token, exp))
    }
}
