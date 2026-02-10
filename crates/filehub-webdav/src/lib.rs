//! WebDAV server implementation for FileHub (RFC 4918).
//!
//! Provides a standards-compliant WebDAV interface allowing clients
//! like Windows Explorer, macOS Finder, Cyberduck, and others to
//! access FileHub storage as a mounted drive.

pub mod auth;
pub mod handler;
pub mod methods;
pub mod properties;
pub mod server;

pub use server::WebDavServer;
