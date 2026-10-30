//! The `static` item of the `refinements` collection: a `static` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `static`, one item of the `refinements` collection.
pub(crate) struct StaticItem;

impl RefinementsItem for StaticItem {
    const TEXT: &'static str = TEXT;
}
