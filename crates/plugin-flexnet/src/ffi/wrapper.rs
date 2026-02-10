//! Safe Rust wrappers around FlexNet FFI bindings.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing;

use super::bindings::{FfiError, FlexNetBindings};

/// Result of a license checkout operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResult {
    /// The checkout token (handle identifier)
    pub token: String,
    /// The feature that was checked out
    pub feature: String,
    /// When the checkout occurred
    pub checked_out_at: DateTime<Utc>,
}

/// Result of a pool status query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatusResult {
    /// Feature name
    pub feature: String,
    /// Total seats available in the license
    pub total_seats: i32,
    /// Currently available seats
    pub available_seats: i32,
    /// Currently checked out seats
    pub checked_out_seats: i32,
    /// When the status was queried
    pub queried_at: DateTime<Utc>,
}

/// Safe wrapper around FlexNet FFI bindings
#[derive(Debug)]
pub struct FlexNetWrapper {
    /// The underlying bindings implementation
    bindings: Arc<dyn FlexNetBindings>,
    /// Whether the library has been initialized
    initialized: std::sync::atomic::AtomicBool,
}

impl FlexNetWrapper {
    /// Create a new wrapper with the given bindings implementation
    pub fn new(bindings: Arc<dyn FlexNetBindings>) -> Self {
        Self {
            bindings,
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Initialize the FlexNet library
    pub fn initialize(&self, license_file: &str) -> Result<(), FfiError> {
        tracing::info!(
            "Initializing FlexNet library with license file: {}",
            license_file
        );
        self.bindings.init(license_file)?;
        self.initialized
            .store(true, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("FlexNet library initialized successfully");
        Ok(())
    }

    /// Shutdown and cleanup the FlexNet library
    pub fn shutdown(&self) -> Result<(), FfiError> {
        if self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("Shutting down FlexNet library");
            self.bindings.cleanup()?;
            self.initialized
                .store(false, std::sync::atomic::Ordering::SeqCst);
            tracing::info!("FlexNet library shut down successfully");
        }
        Ok(())
    }

    /// Check if the library is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Checkout a license for a feature
    pub fn checkout(&self, feature: &str, version: &str) -> Result<CheckoutResult, FfiError> {
        if !self.is_initialized() {
            return Err(FfiError::NotInitialized);
        }

        tracing::debug!("Checking out license for feature '{}'", feature);
        let token = self.bindings.checkout(feature, version)?;

        Ok(CheckoutResult {
            token,
            feature: feature.to_string(),
            checked_out_at: Utc::now(),
        })
    }

    /// Checkin a license by its token
    pub fn checkin(&self, token: &str) -> Result<(), FfiError> {
        if !self.is_initialized() {
            return Err(FfiError::NotInitialized);
        }

        tracing::debug!("Checking in license token '{}'", token);
        self.bindings.checkin(token)?;
        tracing::debug!("License token '{}' checked in successfully", token);
        Ok(())
    }

    /// Get the current pool status for a feature
    pub fn pool_status(&self, feature: &str) -> Result<PoolStatusResult, FfiError> {
        if !self.is_initialized() {
            return Err(FfiError::NotInitialized);
        }

        let total = self.bindings.get_total_seats(feature)?;
        let available = self.bindings.get_available_seats(feature)?;

        Ok(PoolStatusResult {
            feature: feature.to_string(),
            total_seats: total,
            available_seats: available,
            checked_out_seats: total - available,
            queried_at: Utc::now(),
        })
    }

    /// Get the last error from the native library
    pub fn last_error(&self) -> Option<String> {
        self.bindings.get_last_error()
    }
}

impl Drop for FlexNetWrapper {
    fn drop(&mut self) {
        if self.is_initialized() {
            if let Err(e) = self.shutdown() {
                tracing::error!("Error shutting down FlexNet on drop: {}", e);
            }
        }
    }
}
