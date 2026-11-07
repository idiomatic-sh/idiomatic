//! The `variable` item of the `refinements` collection: a single-name `let` binding.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `variable`, one item of the `refinements` collection.
pub(crate) struct VariableItem;

impl RefinementsItem for VariableItem {
    const TEXT: &'static str = TEXT;
}
