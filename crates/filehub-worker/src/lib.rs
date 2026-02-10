//! Background job processing and scheduled tasks for FileHub.
//!
//! This crate provides:
//! - A worker runner that polls for and executes queued jobs
//! - A cron scheduler for periodic maintenance tasks
//! - A job executor that dispatches jobs to the correct handler
//! - Built-in job implementations for cleanup, reports, and maintenance

pub mod executor;
pub mod jobs;
pub mod queue;
pub mod runner;
pub mod scheduler;

pub use runner::WorkerRunner;
pub use scheduler::CronScheduler;
