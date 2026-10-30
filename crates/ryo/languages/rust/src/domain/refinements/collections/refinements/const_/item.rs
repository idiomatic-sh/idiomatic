//! The `const` item of the `refinements` collection: a `const` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `const`, one item of the `refinements` collection.
pub(crate) struct ConstItem;

impl RefinementsItem for ConstItem {
    const TEXT: &'static str = TEXT;
}
