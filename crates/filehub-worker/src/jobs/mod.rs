//! Built-in job handler implementations.

pub mod cleanup;
pub mod conversion;
pub mod license;
pub mod maintenance;
pub mod notification;
pub mod presence;
pub mod report;

pub use cleanup::CleanupJobHandler;
pub use conversion::CadConversionJobHandler;
pub use license::LicenseJobHandler;
pub use maintenance::MaintenanceJobHandler;
pub use notification::NotificationJobHandler;
pub use presence::PresenceJobHandler;
pub use report::ReportJobHandler;
