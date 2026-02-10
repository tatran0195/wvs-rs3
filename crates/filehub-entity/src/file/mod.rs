//! File domain entities.

pub mod chunk;
pub mod metadata;
pub mod model;
pub mod version;

pub use chunk::{ChunkStatus, ChunkedUpload};
pub use metadata::FileMetadata;
pub use model::{CreateFile, File};
pub use version::FileVersion;
