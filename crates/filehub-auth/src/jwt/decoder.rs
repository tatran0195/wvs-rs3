//! JWT token validation and blocklist checking.

use std::sync::Arc;

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use uuid::Uuid;

use filehub_cache::provider::CacheManager;
use filehub_core::config::AuthConfig;
use filehub_core::error::AppError;
use filehub_core::traits::CacheProvider;

use super::claims::{Claims, TokenType};

/// Cache key prefix for blocklisted JWT IDs.
const BLOCKLIST_PREFIX: &str = "jwt:blocklist:";

/// Validates JWT tokens and checks blocklist status.
#[derive(Clone)]
pub struct JwtDecoder {
    /// HMAC secret key for verification.
    decoding_key: DecodingKey,
    /// Validation configuration.
    validation: Validation,
    /// Cache manager for blocklist lookups.
    cache: Arc<CacheManager>,
}

impl std::fmt::Debug for JwtDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtDecoder")
            .field("validation", &self.validation)
            .finish()
    }
}

impl JwtDecoder {
    /// Creates a new decoder from auth configuration.
    pub fn new(config: &AuthConfig, cache: Arc<CacheManager>) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 5; // 5 seconds leeway for clock skew

        Self {
            decoding_key: DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            validation,
            cache,
        }
    }

    /// Decodes and validates an access token string.
    ///
    /// Checks:
    /// 1. Signature validity
    /// 2. Expiration
    /// 3. Token type is Access
    /// 4. JTI not in blocklist
    pub async fn decode_access_token(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.decode_token(token)?;

        if claims.token_type != TokenType::Access {
            return Err(AppError::unauthorized(
                "Invalid token type: expected access token",
            ));
        }

        self.check_blocklist(&claims.jti).await?;

        Ok(claims)
    }

    /// Decodes and validates a refresh token string.
    pub async fn decode_refresh_token(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.decode_token(token)?;

        if claims.token_type != TokenType::Refresh {
            return Err(AppError::unauthorized(
                "Invalid token type: expected refresh token",
            ));
        }

        self.check_blocklist(&claims.jti).await?;

        Ok(claims)
    }

    /// Internal decode without type checking.
    fn decode_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data =
            decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AppError::unauthorized("Token has expired")
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidToken => {
                        AppError::unauthorized("Invalid token format")
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        AppError::unauthorized("Invalid token signature")
                    }
                    _ => AppError::unauthorized(format!("Token validation failed: {e}")),
                }
            })?;

        Ok(token_data.claims)
    }

    /// Checks whether the given JWT ID has been blocklisted.
    async fn check_blocklist(&self, jti: &Uuid) -> Result<(), AppError> {
        let key = format!("{}{}", BLOCKLIST_PREFIX, jti);
        let blocked = self.cache.get(&key).await.ok().flatten();
        if blocked.is_some() {
            return Err(AppError::unauthorized("Token has been revoked"));
        }
        Ok(())
    }

    /// Adds a JWT ID to the blocklist with the remaining TTL.
    pub async fn blocklist_token(
        &self,
        jti: Uuid,
        remaining_ttl_seconds: u64,
    ) -> Result<(), AppError> {
        let key = format!("{}{}", BLOCKLIST_PREFIX, jti);
        let ttl = if remaining_ttl_seconds > 0 {
            std::time::Duration::from_secs(remaining_ttl_seconds)
        } else {
            // Minimum 60 seconds to avoid race conditions
            std::time::Duration::from_secs(60)
        };
        self.cache
            .set(&key, "revoked", ttl)
            .await
            .map_err(|e| AppError::internal(format!("Failed to blocklist token: {e}")))?;
        Ok(())
    }

    /// Blocklists all tokens for a given session by storing a session-level block.
    pub async fn blocklist_session(&self, session_id: Uuid) -> Result<(), AppError> {
        let key = format!("jwt:session_block:{}", session_id);
        // Block for 24 hours (max refresh token lifetime)
        let ttl = std::time::Duration::from_secs(86400);
        self.cache
            .set(&key, "blocked", ttl)
            .await
            .map_err(|e| AppError::internal(format!("Failed to blocklist session: {e}")))?;
        Ok(())
    }

    /// Checks whether a session has been fully blocklisted.
    pub async fn is_session_blocked(&self, session_id: &Uuid) -> Result<bool, AppError> {
        let key = format!("jwt:session_block:{}", session_id);
        let result: Option<String> = self.cache.get(&key).await.unwrap_or(None);
        Ok(result.is_some())
    }
}
