//! The `tuple-field` item of the `refinements` collection: a positional field.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `tuple-field`, one item of the `refinements` collection.
pub(crate) struct TupleFieldItem;

impl RefinementsItem for TupleFieldItem {
    const TEXT: &'static str = TEXT;
}
