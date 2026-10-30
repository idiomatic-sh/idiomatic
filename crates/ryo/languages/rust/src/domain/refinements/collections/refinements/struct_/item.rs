//! The `struct` item of the `refinements` collection: a `struct` item, with named fields, positional fields or none.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `struct`, one item of the `refinements` collection.
pub(crate) struct StructItem;

impl RefinementsItem for StructItem {
    const TEXT: &'static str = TEXT;
}
