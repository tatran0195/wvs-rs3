//! Core traits defined in `filehub-core` and implemented by other crates.

pub mod cache;
pub mod plugin;
pub mod repository;
pub mod seat_allocator;
pub mod service;
pub mod storage;

pub use cache::CacheProvider;
pub use plugin::{HookContext, HookHandler, HookResult, Plugin};
pub use repository::Repository;
pub use seat_allocator::SeatAllocator;
pub use service::Service;
pub use storage::StorageProvider;
