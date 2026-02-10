//! # filehub-entity
//!
//! Domain entity models for Suzuki FileHub. Every struct in this crate
//! represents a database table row or a domain value object. All entities
//! derive `Debug`, `Clone`, `Serialize`, `Deserialize`, and database
//! entities additionally derive `sqlx::FromRow`.

pub mod audit;
pub mod file;
pub mod folder;
pub mod job;
pub mod license;
pub mod notification;
pub mod permission;
pub mod presence;
pub mod session;
pub mod share;
pub mod storage;
pub mod user;
