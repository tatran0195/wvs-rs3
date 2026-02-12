//! Audit log repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::audit::model::{AuditLogEntry, CreateAuditLogEntry};

/// Repository for audit log entries.
#[derive(Debug, Clone)]
pub struct AuditLogRepository {
    pool: PgPool,
}

impl AuditLogRepository {
    /// Create a new audit log repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find an audit entry by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<AuditLogEntry>> {
        sqlx::query_as::<_, AuditLogEntry>("SELECT * FROM audit_log WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find audit entry", e)
            })
    }

    /// Search audit log with filters.
    pub async fn search(
        &self,
        actor_id: Option<Uuid>,
        action: Option<&str>,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        page: &PageRequest,
    ) -> AppResult<PageResponse<AuditLogEntry>> {
        let mut conditions = Vec::new();
        let mut param_idx = 1u32;

        if actor_id.is_some() {
            conditions.push(format!("actor_id = ${param_idx}"));
            param_idx += 1;
        }
        if action.is_some() {
            conditions.push(format!("action = ${param_idx}"));
            param_idx += 1;
        }
        if target_type.is_some() {
            conditions.push(format!("target_type = ${param_idx}"));
            param_idx += 1;
        }
        if target_id.is_some() {
            conditions.push(format!("target_id = ${param_idx}"));
            param_idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM audit_log {where_clause}");
        let select_sql = format!(
            "SELECT * FROM audit_log {where_clause} ORDER BY created_at DESC LIMIT ${param_idx} OFFSET ${}",
            param_idx + 1
        );

        // Build dynamic queries
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        let mut select_query = sqlx::query_as::<_, AuditLogEntry>(&select_sql);

        if let Some(aid) = actor_id {
            count_query = count_query.bind(aid);
            select_query = select_query.bind(aid);
        }
        if let Some(a) = action {
            count_query = count_query.bind(a.to_string());
            select_query = select_query.bind(a.to_string());
        }
        if let Some(tt) = target_type {
            count_query = count_query.bind(tt.to_string());
            select_query = select_query.bind(tt.to_string());
        }
        if let Some(tid) = target_id {
            count_query = count_query.bind(tid);
            select_query = select_query.bind(tid);
        }

        let total = count_query.fetch_one(&self.pool).await.map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to count audit entries", e)
        })?;

        let entries = select_query
            .bind(page.limit() as i64)
            .bind(page.offset() as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to search audit log", e)
            })?;

        Ok(PageResponse::new(
            entries,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Create an audit log entry.
    pub async fn create(&self, data: &CreateAuditLogEntry) -> AppResult<AuditLogEntry> {
        sqlx::query_as::<_, AuditLogEntry>(
            "INSERT INTO audit_log (actor_id, action, target_type, target_id, details, ip_address, user_agent) \
             VALUES ($1, $2, $3, $4, $5, $6::INET, $7) RETURNING *"
        )
            .bind(data.actor_id)
            .bind(&data.action)
            .bind(&data.target_type)
            .bind(data.target_id)
            .bind(&data.details)
            .bind(&data.ip_address)
            .bind(&data.user_agent)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create audit entry", e))
    }

    /// Count occurrences of an action since a specific time.
    pub async fn count_actions_since(
        &self,
        action: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_log WHERE action = $1 AND created_at >= $2",
        )
        .bind(action)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to count audit actions", e)
        })?;
        Ok(count)
    }
    /// Count audit entries since a specific time.
    pub async fn count_since(&self, since: chrono::DateTime<chrono::Utc>) -> AppResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audit_log WHERE created_at >= $1")
                .bind(since)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(
                        ErrorKind::Database,
                        "Failed to count recent audit entries",
                        e,
                    )
                })?;
        Ok(count)
    }

    /// Find since a specific time.
    pub async fn find_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<Vec<AuditLogEntry>> {
        let entries =
            sqlx::query_as::<_, AuditLogEntry>("SELECT * FROM audit_log WHERE created_at >= $1")
                .bind(since)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(
                        ErrorKind::Database,
                        "Failed to find recent audit entries",
                        e,
                    )
                })?;
        Ok(entries)
    }
}
