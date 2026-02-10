//! License checkout logic and validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use filehub_core::types::id::{SessionId, UserId};

/// Parameters for a checkout request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutRequest {
    /// The user requesting the checkout
    pub user_id: UserId,
    /// The session associated with this checkout
    pub session_id: SessionId,
    /// The feature to checkout
    pub feature_name: String,
    /// The client IP address
    pub ip_address: Option<String>,
    /// When the request was made
    pub requested_at: DateTime<Utc>,
}

/// Result of a checkout validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutValidation {
    /// Whether the checkout is allowed
    pub allowed: bool,
    /// Reason if not allowed
    pub reason: Option<String>,
    /// Available seats before checkout
    pub available_seats: i32,
    /// Total seats
    pub total_seats: i32,
}

/// Validates whether a checkout should be allowed
pub fn validate_checkout(
    available_seats: i32,
    admin_reserved: i32,
    is_admin: bool,
) -> CheckoutValidation {
    let effective_available = if is_admin {
        available_seats
    } else {
        (available_seats - admin_reserved).max(0)
    };

    if effective_available <= 0 {
        CheckoutValidation {
            allowed: false,
            reason: Some(if !is_admin && available_seats > 0 {
                "Remaining seats are reserved for administrators".to_string()
            } else {
                "No license seats available".to_string()
            }),
            available_seats,
            total_seats: 0,
        }
    } else {
        CheckoutValidation {
            allowed: true,
            reason: None,
            available_seats,
            total_seats: 0,
        }
    }
}
