//! Checkout token value type.

use serde::{Deserialize, Serialize};

/// A token representing a FlexNet license checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutToken {
    /// The checkout handle string from FlexNet.
    pub handle: String,
    /// The feature name that was checked out.
    pub feature_name: String,
}
