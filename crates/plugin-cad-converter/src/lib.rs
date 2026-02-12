//! # Plugin CAD Converter
//!
//! A FileHub plugin that converts CAD/CAE files to the VTFx visualization
//! format using TechnoStar Jupiter-Web as the conversion engine.
//!
//! ## Jupiter-Web Discovery
//!
//! On Windows, the plugin can auto-detect Jupiter-Web installations by
//! querying the Windows registry for the Inno Setup uninstall GUID
//! `{700798F8-7038-4887-BCC5-37278433D213}`. This eliminates the need
//! to manually configure the `jupiter_path` in most deployments.

pub mod config;
pub mod error;
pub mod filesystem;
pub mod input_resolver;
pub mod jupiter;
pub mod metrics;
pub mod models;
pub mod plugin;
pub mod processor;
pub mod scripting;

pub use config::ConversionConfig;
pub use error::ConversionError;
pub use jupiter::JupiterDiscovery;
pub use models::{ConversionInput, ConversionMode, ConversionOptions, ConversionResult, FileType};
pub use plugin::CadConverterPlugin;
pub use processor::ConversionProcessor;
