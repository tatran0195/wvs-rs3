//! License checkout repository implementation.

use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_entity::license::model::LicenseCheckout;

/// Repository for license checkout records.
#[derive(Debug, Clone)]
pub struct LicenseCheckoutRepository {
    pool: PgPool,
}

impl LicenseCheckoutRepository {
    /// Create a new license checkout repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a checkout by ID.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<LicenseCheckout>> {
        sqlx::query_as::<_, LicenseCheckout>("SELECT * FROM license_checkouts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find checkout", e))
    }

    /// Find active checkouts by session.
    pub async fn find_active_by_session(
        &self,
        session_id: Uuid,
    ) -> AppResult<Vec<LicenseCheckout>> {
        sqlx::query_as::<_, LicenseCheckout>(
            "SELECT * FROM license_checkouts WHERE session_id = $1 AND is_active = TRUE",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find session checkouts", e)
        })
    }

    /// Find all active checkouts.
    pub async fn find_all_active(&self) -> AppResult<Vec<LicenseCheckout>> {
        sqlx::query_as::<_, LicenseCheckout>(
            "SELECT * FROM license_checkouts WHERE is_active = TRUE",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to find active checkouts", e)
        })
    }

    /// Count all active checkouts.
    pub async fn count_active(&self) -> AppResult<u32> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM license_checkouts WHERE is_active = TRUE")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(
                        ErrorKind::Database,
                        "Failed to count active checkouts",
                        e,
                    )
                })?;
        Ok(count as u32)
    }

    /// Create a checkout record.
    pub async fn create(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        feature_name: &str,
        checkout_token: &str,
        ip_address: Option<&str>,
    ) -> AppResult<LicenseCheckout> {
        sqlx::query_as::<_, LicenseCheckout>(
            "INSERT INTO license_checkouts (session_id, user_id, feature_name, checkout_token, ip_address) \
             VALUES ($1, $2, $3, $4, $5::INET) RETURNING *"
        )
            .bind(session_id)
            .bind(user_id)
            .bind(feature_name)
            .bind(checkout_token)
            .bind(ip_address)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to create checkout", e))
    }

    /// Check in a license (set is_active = false).
    pub async fn checkin(&self, checkout_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE license_checkouts SET is_active = FALSE, checked_in_at = NOW() WHERE id = $1",
        )
        .bind(checkout_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to checkin license", e))?;
        Ok(())
    }

    /// Check in all active licenses for a session.
    pub async fn checkin_by_session(&self, session_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query(
            "UPDATE license_checkouts SET is_active = FALSE, checked_in_at = NOW() \
             WHERE session_id = $1 AND is_active = TRUE",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to checkin session licenses", e)
        })?;
        Ok(result.rows_affected())
    }
}
