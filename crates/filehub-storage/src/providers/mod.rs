//! Storage provider implementations.

pub mod local;
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "smb")]
pub mod smb;
#[cfg(feature = "webdav-client")]
pub mod webdav;

pub use local::LocalStorageProvider;
