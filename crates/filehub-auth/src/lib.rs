//! # filehub-auth
//!
//! Complete authentication, authorization, session management, and seat allocation
//! for the Suzuki FileHub platform.
//!
//! ## Modules
//!
//! - `jwt` — JWT token creation, validation, and blocklist management
//! - `password` — Argon2id password hashing and policy enforcement
//! - `session` — Session lifecycle management (create, refresh, terminate)
//! - `rbac` — Role-based access control enforcement
//! - `acl` — Access control list checking with folder inheritance
//! - `seat` — Concurrent session seat allocation and pool management

pub mod acl;
pub mod jwt;
pub mod password;
pub mod rbac;
pub mod seat;
pub mod session;

pub use acl::{AclChecker, AclInheritanceResolver, EffectivePermissionResolver};
pub use jwt::{Claims, JwtDecoder, JwtEncoder};
pub use password::{PasswordHasher, PasswordValidator};
pub use rbac::{RbacEnforcer, RbacPolicies};
pub use seat::{SeatAllocator, SeatReconciler, SessionLimiter};
pub use session::{SessionCleanup, SessionManager, SessionStore};
