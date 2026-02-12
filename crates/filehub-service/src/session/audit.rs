//! Session-related audit logging.

use std::sync::Arc;

use filehub_entity::audit::model::CreateAuditLogEntry;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_database::repositories::audit::AuditLogRepository;
use filehub_entity::audit::AuditLogEntry;

use crate::context::RequestContext;

/// Session and general audit log service.
#[derive(Debug, Clone)]
pub struct SessionAudit {
    /// Audit log repository.
    audit_repo: Arc<AuditLogRepository>,
}

impl SessionAudit {
    /// Creates a new session audit service.
    pub fn new(audit_repo: Arc<AuditLogRepository>) -> Self {
        Self { audit_repo }
    }

    /// Logs an audit event.
    pub async fn log_event(
        &self,
        actor_id: Uuid,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        details: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<AuditLogEntry, AppError> {
        let entry_record = CreateAuditLogEntry {
            actor_id,
            action: action.to_string(),
            target_type: target_type.to_string(),
            target_id,
            details,
            ip_address: ip_address.map(String::from),
            user_agent: user_agent.map(String::from),
        };

        self.audit_repo
            .create(&entry_record)
            .await
            .map_err(|e| AppError::internal(format!("Failed to log audit event: {e}")))
    }

    /// Searches the audit log.
    pub async fn search(
        &self,
        _ctx: &RequestContext,
        actor_id: Option<Uuid>,
        action: Option<&str>,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        page: PageRequest,
    ) -> Result<PageResponse<AuditLogEntry>, AppError> {
        self.audit_repo
            .search(actor_id, action, target_type, target_id, &page)
            .await
            .map_err(|e| AppError::internal(format!("Audit search failed: {e}")))
    }
}
