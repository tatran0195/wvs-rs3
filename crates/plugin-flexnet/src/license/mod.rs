//! License management module for FlexNet integration.

pub mod checkin;
pub mod checkout;
pub mod manager;
pub mod pool;
pub mod reservation;

pub use manager::LicenseManager;
pub use pool::PoolSyncService;
pub use reservation::ReservationManager;
