//! The `function` item of the `refinements` collection: a free function.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `function`, one item of the `refinements` collection.
pub(crate) struct FunctionItem;

impl RefinementsItem for FunctionItem {
    const TEXT: &'static str = TEXT;
}
