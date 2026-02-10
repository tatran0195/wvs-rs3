//! Role-based access control (RBAC) enforcement.

pub mod enforcer;
pub mod policies;

pub use enforcer::RbacEnforcer;
pub use policies::RbacPolicies;
