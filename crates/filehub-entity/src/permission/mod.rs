//! Permission (RBAC + ACL) domain entities.

pub mod acl;
pub mod action;
pub mod model;

pub use acl::{AclInheritance, AclPermission};
pub use action::PermissionAction;
pub use model::AclEntry;
