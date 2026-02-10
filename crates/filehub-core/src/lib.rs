//! # filehub-core
//!
//! Core crate for Suzuki FileHub. Contains traits, configuration schemas,
//! typed identifiers, domain events, pagination/sorting/filter types,
//! and the unified error system.
//!
//! This crate has **no** internal dependencies on other FileHub crates.

pub mod config;
pub mod error;
pub mod events;
pub mod result;
pub mod traits;
pub mod types;

pub use error::AppError;
pub use result::AppResult;
