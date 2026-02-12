//! Newtype wrappers around [`uuid::Uuid`] for all domain entity identifiers.
//!
//! Using distinct types prevents accidentally passing a `UserId` where a
//! `FileId` is expected. When the `sqlx-support` feature is enabled,
//! each ID type also implements `sqlx::Type`, `sqlx::Encode`, and
//! `sqlx::Decode` for PostgreSQL.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Macro to define a newtype ID wrapper around `Uuid`.
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Create a new random identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create an identifier from an existing UUID.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Return the inner UUID value.
            pub fn into_uuid(self) -> Uuid {
                self.0
            }

            /// Return a reference to the inner UUID.
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }

        #[cfg(feature = "sqlx")]
        impl sqlx::Type<sqlx::Postgres> for $name {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                <Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'q> sqlx::Encode<'q, sqlx::Postgres> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <Uuid as sqlx::Encode<'q, sqlx::Postgres>>::encode_by_ref(&self.0, buf)
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $name {
            fn decode(
                value: <sqlx::Postgres as sqlx::Database>::ValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                <Uuid as sqlx::Decode<'r, sqlx::Postgres>>::decode(value).map(Self)
            }
        }
    };
}

define_id!(
    /// Unique identifier for a user.
    UserId
);

define_id!(
    /// Unique identifier for a file.
    FileId
);

define_id!(
    /// Unique identifier for a folder.
    FolderId
);

define_id!(
    /// Unique identifier for a storage backend.
    StorageId
);

define_id!(
    /// Unique identifier for a user session.
    SessionId
);

define_id!(
    /// Unique identifier for a share link or invite.
    ShareId
);

define_id!(
    /// Unique identifier for an ACL entry.
    AclEntryId
);

define_id!(
    /// Unique identifier for a background job.
    JobId
);

define_id!(
    /// Unique identifier for a license checkout.
    LicenseCheckoutId
);

define_id!(
    /// Unique identifier for a notification.
    NotificationId
);

define_id!(
    /// Unique identifier for an audit log entry.
    AuditLogId
);

define_id!(
    /// Unique identifier for a file version.
    FileVersionId
);

define_id!(
    /// Unique identifier for a chunked upload session.
    ChunkedUploadId
);

define_id!(
    /// Unique identifier for an admin broadcast message.
    BroadcastId
);

define_id!(
    /// Unique identifier for a pool snapshot.
    PoolSnapshotId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_new() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_user_id_display() {
        let uuid = Uuid::new_v4();
        let id = UserId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_user_id_from_str() {
        let uuid = Uuid::new_v4();
        let id: UserId = uuid.to_string().parse().expect("should parse");
        assert_eq!(id.0, uuid);
    }

    #[test]
    fn test_serde_roundtrip() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        let parsed: UserId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, parsed);
    }
}
