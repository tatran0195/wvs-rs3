//! CAD file conversion plugin for FileHub.
//!
//! This plugin detects CAD file uploads and queues conversion jobs
//! to transform them into viewable formats (PDF, SVG, PNG).
//! Conversion is performed via external command-line tools.

pub mod converter;
pub mod executor;
pub mod formats;
pub mod plugin;

pub use plugin::CadConverterPlugin;
