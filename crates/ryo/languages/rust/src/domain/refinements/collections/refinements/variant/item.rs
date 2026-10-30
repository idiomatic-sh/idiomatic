//! The `variant` item of the `refinements` collection: a variant of an `enum`.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `variant`, one item of the `refinements` collection.
pub(crate) struct VariantItem;

impl RefinementsItem for VariantItem {
    const TEXT: &'static str = TEXT;
}
