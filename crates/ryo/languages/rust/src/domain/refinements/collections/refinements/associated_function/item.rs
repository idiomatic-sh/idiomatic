//! The `associated-function` item of the `refinements` collection: a function of an `impl` or a `trait` without a receiver.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `associated-function`, one item of the `refinements` collection.
pub(crate) struct AssociatedFunctionItem;

impl RefinementsItem for AssociatedFunctionItem {
    const TEXT: &'static str = TEXT;
}
