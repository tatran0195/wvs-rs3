//! Storage domain entities.

pub mod model;
pub mod provider;
pub mod quota;

pub use model::{CreateStorage, Storage};
pub use provider::StorageProviderType;
pub use quota::StorageQuota;
