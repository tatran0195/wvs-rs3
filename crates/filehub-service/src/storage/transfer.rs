//! Cross-storage file transfer service.

use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use filehub_auth::rbac::RbacEnforcer;
use filehub_auth::rbac::policies::SystemPermission;
use filehub_core::error::AppError;
use filehub_storage::manager::StorageManager;

use crate::context::RequestContext;

/// Handles file transfers between storage backends.
#[derive(Clone)]
pub struct TransferService {
    /// Storage manager.
    storage: Arc<StorageManager>,
    /// RBAC enforcer.
    rbac: Arc<RbacEnforcer>,
}

impl std::fmt::Debug for TransferService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferService").finish()
    }
}

/// Request to transfer a file between storages.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferRequest {
    /// Source storage ID.
    pub source_storage_id: Uuid,
    /// Source path.
    pub source_path: String,
    /// Target storage ID.
    pub target_storage_id: Uuid,
    /// Target path.
    pub target_path: String,
}

impl TransferService {
    /// Creates a new transfer service.
    pub fn new(storage: Arc<StorageManager>, rbac: Arc<RbacEnforcer>) -> Self {
        Self { storage, rbac }
    }

    /// Transfers a file between storage backends.
    pub async fn transfer(
        &self,
        ctx: &RequestContext,
        req: TransferRequest,
    ) -> Result<(), AppError> {
        self.rbac
            .require_permission(&ctx.role, &SystemPermission::StorageTransfer)?;

        // Read from source
        let data = self
            .storage
            .read(&req.source_storage_id, &req.source_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to read from source: {e}")))?;

        // Write to target
        self.storage
            .write(&req.target_storage_id, &req.target_path, data)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write to target: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            source = %req.source_storage_id,
            target = %req.target_storage_id,
            "Cross-storage transfer completed"
        );

        Ok(())
    }
}
