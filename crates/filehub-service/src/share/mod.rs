//! Share management — create, validate, and access shared resources.

pub mod access;
pub mod link;
pub mod service;

pub use access::AccessService;
pub use link::LinkService;
pub use service::ShareService;
