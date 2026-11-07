//! The `label` item of the `refinements` collection: a label of a loop or a block, `'outer: loop {}`.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `label`, one item of the `refinements` collection.
pub(crate) struct LabelItem;

impl RefinementsItem for LabelItem {
    const TEXT: &'static str = TEXT;
}
