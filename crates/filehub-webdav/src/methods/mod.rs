//! WebDAV method implementations.

pub mod copy;
pub mod get_put;
pub mod lock;
pub mod mkcol;
pub mod move_op;
pub mod propfind;
pub mod proppatch;

pub use copy::handle_copy;
pub use get_put::{handle_get, handle_head, handle_put};
pub use lock::{handle_lock, handle_unlock};
pub use mkcol::handle_mkcol;
pub use move_op::handle_move;
pub use propfind::handle_propfind;
pub use proppatch::handle_proppatch;
