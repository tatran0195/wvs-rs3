//! # filehub-storage
//!
//! Storage provider implementations for FileHub. Supports local filesystem,
//! S3-compatible object stores, WebDAV (client), and SMB shares.

pub mod chunked;
pub mod manager;
pub mod providers;
pub mod thumbnail;
pub mod transfer;

pub use manager::StorageManager;
