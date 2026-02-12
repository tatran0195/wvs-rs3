//! # filehub-service
//!
//! Business logic service layer for FileHub. Each service orchestrates
//! repositories, cache, storage providers, and authentication to implement
//! application-level use cases.
//!
//! Services follow constructor injection — all dependencies are provided
//! at construction time via `Arc` references.

pub mod context;
pub mod file;
pub mod folder;
pub mod notification;
pub mod permission;
pub mod report;
pub mod session;
pub mod share;
pub mod storage;
pub mod user;

pub use context::RequestContext;
pub use file::{
    DownloadService, FileService, PreviewService, SearchService, UploadService, VersionService,
};
pub use folder::{FolderService, TreeService};
pub use notification::{NotificationRules, NotificationService};
pub use permission::PermissionService;
pub use report::WeeklyReportService;
pub use session::{SessionAudit, SessionService, TerminationService};
pub use share::{AccessService, LinkService, ShareService};
pub use storage::{StorageService, TransferService};
pub use user::{AdminUserService, UserService};
