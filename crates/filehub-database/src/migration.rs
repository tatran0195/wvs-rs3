//! Database migration runner.

use sqlx::PgPool;
use tracing::info;

use filehub_core::error::{AppError, ErrorKind};

/// Run all pending database migrations.
pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    info!("Running database migrations...");

    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                format!("Failed to run migrations: {e}"),
                e,
            )
        })?;

    info!("Database migrations completed successfully");
    Ok(())
}
