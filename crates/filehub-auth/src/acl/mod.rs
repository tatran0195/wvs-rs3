//! Access control list (ACL) checking with folder inheritance and effective permission resolution.

pub mod checker;
pub mod inheritance;
pub mod resolver;

pub use checker::AclChecker;
pub use inheritance::AclInheritanceResolver;
pub use resolver::EffectivePermissionResolver;
