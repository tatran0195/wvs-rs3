//! Event subscription API for plugins.

use crate::hooks::definitions::HookPoint;

/// Describes a hook subscription request from a plugin.
#[derive(Debug, Clone)]
pub struct HookSubscription {
    /// The hook point to subscribe to.
    pub hook: HookPoint,
    /// Priority (lower = runs earlier).
    pub priority: i32,
}

impl HookSubscription {
    /// Creates a new hook subscription.
    pub fn new(hook: HookPoint, priority: i32) -> Self {
        Self { hook, priority }
    }

    /// Creates a subscription with default priority (100).
    pub fn default_priority(hook: HookPoint) -> Self {
        Self {
            hook,
            priority: 100,
        }
    }
}
