//! License checkout entity model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A record of a FlexNet license checkout.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LicenseCheckout {
    /// Unique checkout identifier.
    pub id: Uuid,
    /// Session that holds this checkout.
    pub session_id: Option<Uuid>,
    /// User who checked out the license.
    pub user_id: Uuid,
    /// Licensed feature name.
    pub feature_name: String,
    /// FlexNet checkout handle/token.
    pub checkout_token: String,
    /// When the license was checked out.
    pub checked_out_at: DateTime<Utc>,
    /// When the license was checked in (None = still active).
    pub checked_in_at: Option<DateTime<Utc>>,
    /// IP address from which the checkout was made.
    pub ip_address: Option<String>,
    /// Whether this checkout is still active.
    pub is_active: Option<bool>,
}

impl LicenseCheckout {
    /// Check if this checkout is currently active.
    pub fn is_currently_active(&self) -> bool {
        self.is_active.unwrap_or(false) && self.checked_in_at.is_none()
    }
}
