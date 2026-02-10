//! Filter types for dynamic query building.

use serde::{Deserialize, Serialize};

/// Filter comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    /// Exact equality.
    Eq,
    /// Not equal.
    Ne,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Gte,
    /// Less than.
    Lt,
    /// Less than or equal.
    Lte,
    /// SQL `LIKE` pattern match.
    Like,
    /// SQL `ILIKE` case-insensitive pattern match.
    ILike,
    /// SQL `IN` list membership.
    In,
    /// SQL `IS NULL` check.
    IsNull,
    /// SQL `IS NOT NULL` check.
    IsNotNull,
}

/// A dynamic filter value that can represent various SQL types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Boolean(bool),
    /// A list of string values (for `IN` operator).
    StringList(Vec<String>),
    /// Null / no value (for `IS NULL`, `IS NOT NULL`).
    Null,
}

/// A single filter condition on a named field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterField {
    /// The column or field name to filter on.
    pub field: String,
    /// The comparison operator.
    pub op: FilterOp,
    /// The value to compare against.
    pub value: FilterValue,
}

impl FilterField {
    /// Create a new filter field.
    pub fn new(field: impl Into<String>, op: FilterOp, value: FilterValue) -> Self {
        Self {
            field: field.into(),
            op,
            value,
        }
    }

    /// Shorthand for an equality filter.
    pub fn eq(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOp::Eq, FilterValue::String(value.into()))
    }

    /// Shorthand for a case-insensitive LIKE filter.
    pub fn ilike(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self::new(field, FilterOp::ILike, FilterValue::String(pattern.into()))
    }
}
