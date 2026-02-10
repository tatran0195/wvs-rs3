//! Session limit resolution types.

use serde::{Deserialize, Serialize};

/// Resolved session limit for a user.
///
/// Session limits are resolved in priority order:
/// 1. Per-user override (from `user_session_limits` table)
/// 2. Per-role configuration (from `session.limits.by_role` config)
/// 3. Default (unlimited, bounded only by license pool)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionLimit {
    /// A fixed maximum number of concurrent sessions.
    Fixed(u32),
    /// No per-user limit; bounded only by the global license pool.
    Unlimited,
}

impl SessionLimit {
    /// Check whether a given active session count exceeds this limit.
    pub fn is_exceeded_by(&self, active_count: u32) -> bool {
        match self {
            Self::Fixed(max) => active_count >= *max,
            Self::Unlimited => false,
        }
    }

    /// Return the numeric limit, or `None` for unlimited.
    pub fn as_max(&self) -> Option<u32> {
        match self {
            Self::Fixed(max) => Some(*max),
            Self::Unlimited => None,
        }
    }
}

impl From<u32> for SessionLimit {
    /// Convert a `u32` to a `SessionLimit`. `0` means unlimited.
    fn from(value: u32) -> Self {
        if value == 0 {
            Self::Unlimited
        } else {
            Self::Fixed(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_limit() {
        let limit = SessionLimit::Fixed(3);
        assert!(!limit.is_exceeded_by(2));
        assert!(limit.is_exceeded_by(3));
        assert!(limit.is_exceeded_by(4));
    }

    #[test]
    fn test_unlimited() {
        let limit = SessionLimit::Unlimited;
        assert!(!limit.is_exceeded_by(0));
        assert!(!limit.is_exceeded_by(100));
        assert!(!limit.is_exceeded_by(u32::MAX));
    }

    #[test]
    fn test_from_u32() {
        assert_eq!(SessionLimit::from(0), SessionLimit::Unlimited);
        assert_eq!(SessionLimit::from(5), SessionLimit::Fixed(5));
    }
}
