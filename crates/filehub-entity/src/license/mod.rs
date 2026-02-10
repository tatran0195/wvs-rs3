//! License domain entities.

pub mod checkout;
pub mod model;
pub mod pool;

pub use checkout::CheckoutToken;
pub use model::LicenseCheckout;
pub use pool::{PoolSnapshot, PoolStatus};
