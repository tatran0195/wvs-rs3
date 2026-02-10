//! WebSocket connection management — lifecycle, pool, handles, heartbeat, auth.

pub mod authenticator;
pub mod handle;
pub mod heartbeat;
pub mod manager;
pub mod pool;

pub use handle::ConnectionHandle;
pub use manager::ConnectionManager;
