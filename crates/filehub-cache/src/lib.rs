//! # filehub-cache
//!
//! Cache provider implementations for FileHub. Supports three modes:
//!
//! - **memory**: In-process cache using [moka](https://crates.io/crates/moka)
//! - **redis**: Redis-backed cache using the [redis](https://crates.io/crates/redis) crate
//! - **layered**: L1 in-memory + L2 Redis (future)
//!
//! The provider is selected at runtime based on configuration.

pub mod keys;
#[cfg(feature = "memory")]
pub mod memory;
pub mod provider;
#[cfg(feature = "redis-backend")]
pub mod redis;

pub use provider::CacheManager;
