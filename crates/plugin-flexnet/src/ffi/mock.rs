//! Mock implementation of the license manager for development and testing.
//!
//! Simulates the behavior of `license_proxy.dll` without requiring
//! a license server or DLL file.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tracing;

/// Mock license manager that simulates DLL behavior in-memory.
#[derive(Debug)]
pub struct MockLicenseManager {
    /// Whether initialized
    initialized: Mutex<bool>,
    /// Active checkouts: feature -> set of session_ids
    checkouts: Mutex<HashMap<String, HashSet<String>>>,
    /// Total seats per feature (configurable for testing)
    total_seats: Mutex<HashMap<String, i32>>,
    /// Whether this is a star (unlimited) license
    is_star: Mutex<bool>,
    /// Simulated server info string
    server_info: Mutex<String>,
}

impl MockLicenseManager {
    /// Create a new mock license manager
    pub fn new() -> Self {
        Self {
            initialized: Mutex::new(false),
            checkouts: Mutex::new(HashMap::new()),
            total_seats: Mutex::new(HashMap::new()),
            is_star: Mutex::new(false),
            server_info: Mutex::new("MockServer@localhost".to_string()),
        }
    }

    /// Set the total seats for a feature (for testing)
    pub fn set_total_seats(&self, feature: &str, seats: i32) {
        let mut total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
        total.insert(feature.to_string(), seats);
    }

    /// Set whether this is a star license
    pub fn set_star_license(&self, is_star: bool) {
        let mut star = self.is_star.lock().unwrap_or_else(|e| e.into_inner());
        *star = is_star;
    }

    /// Initialize the mock license manager
    pub fn initialize(&self, _override_path: Option<&str>) -> i32 {
        let mut init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
        *init = true;
        tracing::info!("[MockLM] Initialized");
        0 // LM_SUCCESS
    }

    /// Checkout a feature for a session
    pub fn checkout(&self, feature: &str, session_id: &str) -> i32 {
        let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
        if !*init {
            tracing::error!("[MockLM] Not initialized");
            return -1;
        }
        drop(init);

        // Check star license — unlimited
        let is_star = *self.is_star.lock().unwrap_or_else(|e| e.into_inner());

        let total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
        let max_seats = total.get(feature).copied().unwrap_or(10);
        drop(total);

        let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
        let sessions = checkouts
            .entry(feature.to_string())
            .or_insert_with(HashSet::new);

        // Already checked out for this session — idempotent
        if sessions.contains(session_id) {
            tracing::debug!(
                "[MockLM] Session '{}' already checked out for '{}'",
                session_id,
                feature
            );
            return 0;
        }

        if !is_star && sessions.len() as i32 >= max_seats {
            tracing::warn!(
                "[MockLM] Checkout denied: no seats available for '{}' ({}/{})",
                feature,
                sessions.len(),
                max_seats
            );
            return -1;
        }

        sessions.insert(session_id.to_string());
        tracing::info!(
            "[MockLM] Checked out '{}' for session '{}' ({}/{})",
            feature,
            session_id,
            sessions.len(),
            if is_star {
                "∞".to_string()
            } else {
                max_seats.to_string()
            }
        );
        0 // LM_SUCCESS
    }

    /// Checkin a feature for a session
    pub fn checkin(&self, feature: &str, session_id: &str) -> i32 {
        let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sessions) = checkouts.get_mut(feature) {
            if sessions.remove(session_id) {
                tracing::info!(
                    "[MockLM] Checked in '{}' for session '{}' (remaining: {})",
                    feature,
                    session_id,
                    sessions.len()
                );
                return 0;
            }
        }
        tracing::warn!(
            "[MockLM] Checkin: session '{}' not found for '{}'",
            session_id,
            feature
        );
        -1
    }

    /// Get token pool info: (total, used)
    pub fn get_token_pool(&self, feature: &str) -> (i32, i32, i32) {
        let init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
        if !*init {
            return (-1, 0, 0);
        }
        drop(init);

        let total = self.total_seats.lock().unwrap_or_else(|e| e.into_inner());
        let max_seats = total.get(feature).copied().unwrap_or(10);
        drop(total);

        let checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
        let used = checkouts.get(feature).map(|s| s.len() as i32).unwrap_or(0);

        (0, max_seats, used) // (result, total, used)
    }

    /// Check if star license
    pub fn is_star_license(&self) -> bool {
        *self.is_star.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Get server info
    pub fn get_server_info(&self) -> String {
        self.server_info
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Release all checkouts
    pub fn release_all(&self) {
        let mut checkouts = self.checkouts.lock().unwrap_or_else(|e| e.into_inner());
        let total_released: usize = checkouts.values().map(|s| s.len()).sum();
        checkouts.clear();
        tracing::info!("[MockLM] Released all ({} checkouts)", total_released);
    }

    /// Destroy / cleanup
    pub fn destroy(&self) {
        self.release_all();
        let mut init = self.initialized.lock().unwrap_or_else(|e| e.into_inner());
        *init = false;
        tracing::info!("[MockLM] Destroyed");
    }
}

impl Default for MockLicenseManager {
    fn default() -> Self {
        Self::new()
    }
}
