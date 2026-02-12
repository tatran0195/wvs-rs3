//! Share CRUD service.

use std::sync::Arc;

use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::password::PasswordHasher;
use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::share::ShareRepository;
use filehub_entity::permission::AclPermission;
use filehub_entity::share::{CreateShare, Share, ShareType};

use super::link::LinkService;
use crate::context::RequestContext;

/// Manages share creation, listing, and revocation.
#[derive(Debug, Clone)]
pub struct ShareService {
    /// Share repository.
    share_repo: Arc<ShareRepository>,
    /// Link service for token generation.
    link_service: Arc<LinkService>,
    /// Password hasher for password-protected shares.
    hasher: Arc<PasswordHasher>,
}

/// Request to create a new share.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateShareRequest {
    /// Share type.
    pub share_type: ShareType,
    /// Resource type being shared.
    pub resource_type: filehub_entity::permission::ResourceType,
    /// Resource ID being shared.
    pub resource_id: Uuid,
    /// Password protection (optional).
    pub password: Option<String>,
    /// Target user ID (for user_share type).
    pub shared_with: Option<Uuid>,
    /// Permission level.
    pub permission: AclPermission,
    /// Allow download.
    pub allow_download: bool,
    /// Maximum downloads (None = unlimited).
    pub max_downloads: Option<i32>,
    /// Expiration time (optional).
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

/// Request to update an existing share.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateShareRequest {
    /// Update permission.
    pub permission: Option<AclPermission>,
    /// Update download permission.
    pub allow_download: Option<bool>,
    /// Update max downloads.
    pub max_downloads: Option<Option<i32>>,
    /// Update expiration.
    pub expires_at: Option<Option<chrono::DateTime<Utc>>>,
    /// Update active state.
    pub is_active: Option<bool>,
}

impl ShareService {
    /// Creates a new share service.
    pub fn new(
        share_repo: Arc<ShareRepository>,
        link_service: Arc<LinkService>,
        hasher: Arc<PasswordHasher>,
    ) -> Self {
        Self {
            share_repo,
            link_service,
            hasher,
        }
    }

    /// Lists shares created by the current user.
    pub async fn list_shares(
        &self,
        ctx: &RequestContext,
        page: PageRequest,
    ) -> Result<PageResponse<Share>, AppError> {
        self.share_repo
            .find_by_creator(ctx.user_id, &page)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list shares: {e}")))
    }

    /// Creates a new share.
    pub async fn create_share(
        &self,
        ctx: &RequestContext,
        req: CreateShareRequest,
    ) -> Result<Share, AppError> {
        let token = match req.share_type {
            ShareType::PublicLink | ShareType::PrivateLink => {
                Some(self.link_service.generate_token())
            }
            ShareType::UserShare => None,
        };

        let password_hash = if let Some(ref password) = req.password {
            Some(self.hasher.hash_password(password)?)
        } else {
            None
        };

        if req.share_type == ShareType::UserShare && req.shared_with.is_none() {
            return Err(AppError::validation(
                "shared_with is required for user shares",
            ));
        }

        let share = CreateShare {
            share_type: req.share_type,
            resource_type: req.resource_type,
            resource_id: req.resource_id,
            password_hash,
            token,
            shared_with: req.shared_with,
            permission: req.permission,
            allow_download: req.allow_download,
            max_downloads: req.max_downloads,
            expires_at: req.expires_at,
            created_by: ctx.user_id,
        };

        let share = self
            .share_repo
            .create(&share)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create share: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            share_id = %share.id,
            share_type = ?share.share_type,
            "Share created"
        );

        Ok(share)
    }

    /// Gets a share by ID (only creator or admin can view).
    pub async fn get_share(&self, ctx: &RequestContext, share_id: Uuid) -> Result<Share, AppError> {
        let share = self
            .share_repo
            .find_by_id(share_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Share not found"))?;

        if share.created_by != ctx.user_id && !ctx.is_admin() {
            return Err(AppError::forbidden("You can only view your own shares"));
        }

        Ok(share)
    }

    /// Updates a share.
    pub async fn update_share(
        &self,
        ctx: &RequestContext,
        share_id: Uuid,
        req: UpdateShareRequest,
    ) -> Result<Share, AppError> {
        let mut share = self.get_share(ctx, share_id).await?;

        share.allow_download = req.allow_download;
        share.is_active = req.is_active;

        if let Some(permission) = req.permission {
            share.permission = permission;
        }
        if let Some(max_downloads) = req.max_downloads {
            share.max_downloads = max_downloads;
        }
        if let Some(expires_at) = req.expires_at {
            share.expires_at = expires_at;
        }

        self.share_repo
            .update(&share)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update share: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            share_id = %share_id,
            "Share updated"
        );

        Ok(share)
    }

    /// Revokes (deactivates) a share.
    pub async fn revoke_share(&self, ctx: &RequestContext, share_id: Uuid) -> Result<(), AppError> {
        let share = self.get_share(ctx, share_id).await?;

        self.share_repo
            .deactivate(share.id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to revoke share: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            share_id = %share_id,
            "Share revoked"
        );

        Ok(())
    }
}
