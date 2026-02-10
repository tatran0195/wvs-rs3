//! Service marker trait.

/// Marker trait for business logic services.
///
/// All services in `filehub-service` implement this trait to enable
/// uniform lifecycle management and dependency injection.
pub trait Service: Send + Sync + 'static {}
