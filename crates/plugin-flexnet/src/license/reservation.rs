//! Admin seat reservation management.

use serde::{Deserialize, Serialize};
use tracing;

/// Configuration for admin seat reservations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationConfig {
    /// Whether admin reservation is enabled
    pub enabled: bool,
    /// Number of seats reserved for admins
    pub reserved_seats: i32,
}

/// Manages admin seat reservations.
///
/// When enabled, a number of seats are reserved exclusively for admin users.
/// Non-admin users cannot checkout if only reserved seats remain.
#[derive(Debug, Clone)]
pub struct ReservationManager {
    /// Reservation configuration
    config: ReservationConfig,
}

impl ReservationManager {
    /// Create a new reservation manager
    pub fn new(config: ReservationConfig) -> Self {
        Self { config }
    }

    /// Check if admin reservation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the number of reserved seats
    pub fn reserved_seats(&self) -> i32 {
        if self.config.enabled {
            self.config.reserved_seats
        } else {
            0
        }
    }

    /// Calculate effective available seats for non-admin users.
    ///
    /// Subtracts reserved seats from the total available.
    pub fn effective_available(&self, total_available: i32) -> i32 {
        if self.config.enabled {
            (total_available - self.config.reserved_seats).max(0)
        } else {
            total_available
        }
    }

    /// Calculate effective available seats for admin users (no restriction)
    pub fn effective_available_for_admin(&self, total_available: i32) -> i32 {
        total_available
    }

    /// Update reservation configuration
    pub fn update_config(&mut self, config: ReservationConfig) {
        tracing::info!(
            "Updating reservation config: enabled={}, reserved={}",
            config.enabled,
            config.reserved_seats
        );
        self.config = config;
    }
}
