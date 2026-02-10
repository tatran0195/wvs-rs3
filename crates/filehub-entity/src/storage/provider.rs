//! Storage provider type enumeration.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The type of storage backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storage_provider_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StorageProviderType {
    /// Local filesystem.
    Local,
    /// S3-compatible object storage.
    S3,
    /// WebDAV remote storage.
    Webdav,
    /// SMB/CIFS network share.
    Smb,
}

impl StorageProviderType {
    /// Return the provider type as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
            Self::Webdav => "webdav",
            Self::Smb => "smb",
        }
    }
}

impl fmt::Display for StorageProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for StorageProviderType {
    type Err = filehub_core::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "s3" => Ok(Self::S3),
            "webdav" => Ok(Self::Webdav),
            "smb" => Ok(Self::Smb),
            _ => Err(filehub_core::AppError::validation(format!(
                "Invalid storage provider type: '{s}'. Expected one of: local, s3, webdav, smb"
            ))),
        }
    }
}
