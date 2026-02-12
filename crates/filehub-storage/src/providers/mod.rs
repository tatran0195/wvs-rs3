//! Storage provider implementations.

pub mod local;
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "smb")]
pub mod smb;

pub use local::LocalStorageProvider;
