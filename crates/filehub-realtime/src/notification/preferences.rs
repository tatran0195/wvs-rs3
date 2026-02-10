//! User notification preference checking.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Per-category settings
    pub categories: HashMap<String, CategoryPreference>,
    /// Global mute
    pub muted: bool,
    /// Do not disturb mode
    pub dnd: bool,
}

/// Preference for a notification category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPreference {
    /// Whether this category is enabled
    pub enabled: bool,
    /// Minimum priority to show ("low", "normal", "high", etc.)
    pub min_priority: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            categories: HashMap::new(),
            muted: false,
            dnd: false,
        }
    }
}

impl UserPreferences {
    /// Check if a notification should be delivered based on preferences
    pub fn should_deliver(&self, category: &str, priority: &str) -> bool {
        if self.muted {
            return false;
        }

        if self.dnd {
            // Only deliver critical in DND mode
            return priority == "critical" || priority == "urgent";
        }

        if let Some(pref) = self.categories.get(category) {
            if !pref.enabled {
                return false;
            }
            priority_gte(priority, &pref.min_priority)
        } else {
            true // Default: deliver if no preference set
        }
    }
}

/// Compare priorities: is `a` >= `b`?
fn priority_gte(a: &str, b: &str) -> bool {
    let val = |p: &str| -> i32 {
        match p {
            "low" => 0,
            "normal" => 1,
            "high" => 2,
            "urgent" => 3,
            "critical" => 4,
            _ => 1,
        }
    };
    val(a) >= val(b)
}
