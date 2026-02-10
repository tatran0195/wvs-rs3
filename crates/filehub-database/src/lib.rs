//! # filehub-database
//!
//! PostgreSQL database connection management and concrete repository
//! implementations for all FileHub entities.

pub mod connection;
pub mod migration;
pub mod repositories;

pub use connection::DatabasePool;
