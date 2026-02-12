//! Share access control — validates share tokens and enforces share restrictions.

use std::sync::Arc;

use chrono::Utc;

use filehub_auth::password::PasswordHasher;
use filehub_core::error::AppError;
use filehub_database::repositories::share::ShareRepository;
use filehub_entity::share::Share;

/// Handles public share access validation.
#[derive(Debug, Clone)]
pub struct AccessService {
    /// Share repository.
    share_repo: Arc<ShareRepository>,
    /// Password hasher for verification.
    hasher: Arc<PasswordHasher>,
}

impl AccessService {
    /// Creates a new access service.
    pub fn new(share_repo: Arc<ShareRepository>, hasher: Arc<PasswordHasher>) -> Self {
        Self { share_repo, hasher }
    }

    /// Validates a share token and returns the share if valid.
    pub async fn validate_token(&self, token: &str) -> Result<Share, AppError> {
        let share = self
            .share_repo
            .find_by_token(token)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Invalid or expired share link"))?;

        self.validate_share(&share)?;

        // Update last accessed
        let _ = self
            .share_repo
            .update_last_accessed(share.id, Utc::now())
            .await;

        Ok(share)
    }

    /// Verifies a password for a password-protected share.
    pub async fn verify_password(&self, token: &str, password: &str) -> Result<Share, AppError> {
        let share = self.validate_token(token).await?;

        if let Some(ref hash) = share.password_hash {
            let valid = self.hasher.verify_password(password, hash)?;
            if !valid {
                return Err(AppError::unauthorized("Invalid share password"));
            }
        }

        Ok(share)
    }

    /// Records a download against a share (for download counting).
    pub async fn record_download(&self, share_id: uuid::Uuid) -> Result<i32, AppError> {
        self.share_repo
            .increment_download_count(share_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to record download: {e}")))
    }

    /// Validates share is active, not expired, and within download limits.
    fn validate_share(&self, share: &Share) -> Result<(), AppError> {
        if share.is_active.unwrap_or(false) {
            return Err(AppError::not_found("Share link has been deactivated"));
        }

        if let Some(expires) = share.expires_at {
            if expires <= Utc::now() {
                return Err(AppError::not_found("Share link has expired"));
            }
        }

        if let Some(max) = share.max_downloads {
            if share.download_count.unwrap_or(0) >= max {
                return Err(AppError::not_found(
                    "Share link has reached its download limit",
                ));
            }
        }

        Ok(())
    }
}
