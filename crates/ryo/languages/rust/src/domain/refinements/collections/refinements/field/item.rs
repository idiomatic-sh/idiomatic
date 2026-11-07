//! The `field` item of the `refinements` collection: a named field.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `field`, one item of the `refinements` collection.
pub(crate) struct FieldItem;

impl RefinementsItem for FieldItem {
    const TEXT: &'static str = TEXT;
}
