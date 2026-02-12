//! PostgreSQL connection pool management.

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

use filehub_core::config::DatabaseConfig;
use filehub_core::error::{AppError, ErrorKind};

/// Wrapper around the sqlx PostgreSQL connection pool.
#[derive(Debug, Clone)]
pub struct DatabasePool {
    /// The underlying sqlx connection pool.
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database pool from configuration.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self, AppError> {
        info!(
            url = %mask_password(&config.url),
            max_connections = config.max_connections,
            min_connections = config.min_connections,
            "Connecting to PostgreSQL"
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .connect(&config.url)
            .await
            .map_err(|e| {
                AppError::with_source(
                    ErrorKind::Database,
                    format!("Failed to connect to database: {e}"),
                    e,
                )
            })?;

        info!("Successfully connected to PostgreSQL");
        Ok(Self { pool })
    }

    /// Return a reference to the underlying sqlx pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Return the underlying sqlx pool (consuming self).
    pub fn into_pool(self) -> PgPool {
        self.pool
    }

    /// Check database connectivity.
    pub async fn health_check(&self) -> Result<bool, AppError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|v| v == 1)
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Health check failed", e))
    }

    /// Close all connections in the pool.
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database pool closed");
    }
}

/// Mask the password portion of a database URL for safe logging.
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
            if colon_pos > scheme_end {
                return format!("{}:****@{}", &url[..colon_pos], &url[at_pos + 1..]);
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        assert_eq!(
            mask_password("postgres://user:secret@localhost:5432/db"),
            "postgres://user:****@localhost:5432/db"
        );
        assert_eq!(
            mask_password("postgres://localhost:5432/db"),
            "postgres://localhost:5432/db"
        );
    }
}
