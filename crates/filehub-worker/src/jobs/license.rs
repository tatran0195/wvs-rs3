//! License pool sync, reconciliation, and idle release jobs.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use filehub_entity::job::model::Job;

use crate::executor::{JobExecutionError, JobHandler};

/// Trait for license pool operations — decouples from plugin-flexnet
#[async_trait]
pub trait LicensePoolService: Send + Sync + std::fmt::Debug {
    /// Sync pool status from license server
    async fn sync_pool(&self) -> Result<Value, filehub_core::error::AppError>;

    /// Reconcile pool state
    async fn reconcile(&self) -> Result<Value, filehub_core::error::AppError>;

    /// Check if license system is enabled
    fn is_enabled(&self) -> bool;
}

/// Handles license-related background jobs
#[derive(Debug)]
pub struct LicenseJobHandler {
    /// License pool service
    pool_service: Option<Arc<dyn LicensePoolService>>,
}

impl LicenseJobHandler {
    /// Create a new license job handler
    pub fn new(pool_service: Option<Arc<dyn LicensePoolService>>) -> Self {
        Self { pool_service }
    }

    /// Sync pool status
    async fn pool_sync(&self) -> Result<Value, JobExecutionError> {
        let service = self.pool_service.as_ref().ok_or_else(|| {
            JobExecutionError::Permanent("License system not enabled".to_string())
        })?;

        if !service.is_enabled() {
            return Ok(serde_json::json!({
                "task": "pool_sync",
                "status": "skipped",
                "reason": "license system disabled",
            }));
        }

        let result = service
            .sync_pool()
            .await
            .map_err(|e| JobExecutionError::Transient(format!("Pool sync failed: {}", e)))?;

        Ok(result)
    }

    /// Reconcile pool
    async fn reconcile(&self) -> Result<Value, JobExecutionError> {
        let service = self.pool_service.as_ref().ok_or_else(|| {
            JobExecutionError::Permanent("License system not enabled".to_string())
        })?;

        if !service.is_enabled() {
            return Ok(serde_json::json!({
                "task": "pool_reconciliation",
                "status": "skipped",
                "reason": "license system disabled",
            }));
        }

        let result = service.reconcile().await.map_err(|e| {
            JobExecutionError::Transient(format!("Pool reconciliation failed: {}", e))
        })?;

        Ok(result)
    }
}

#[async_trait]
impl JobHandler for LicenseJobHandler {
    fn job_type(&self) -> &str {
        "pool_sync"
    }

    async fn execute(&self, job: &Job) -> Result<Option<Value>, JobExecutionError> {
        let task = job
            .payload
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("pool_sync");

        let result = match task {
            "pool_sync" => self.pool_sync().await?,
            "pool_reconciliation" => self.reconcile().await?,
            _ => {
                return Err(JobExecutionError::Permanent(format!(
                    "Unknown license task: '{}'",
                    task
                )));
            }
        };

        Ok(Some(result))
    }
}
