//! Sorting types for list endpoints.

use serde::{Deserialize, Serialize};

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl SortDirection {
    /// Return the SQL keyword for this direction.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// A sort specification consisting of a field name and direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortField {
    /// Column or field name to sort by.
    pub field: String,
    /// Sort direction.
    #[serde(default)]
    pub direction: SortDirection,
}

impl SortField {
    /// Create a new sort field.
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            direction,
        }
    }

    /// Create an ascending sort on the given field.
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Asc)
    }

    /// Create a descending sort on the given field.
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Desc)
    }
}
