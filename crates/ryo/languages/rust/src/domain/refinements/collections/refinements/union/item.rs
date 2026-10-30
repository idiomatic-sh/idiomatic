//! The `union` item of the `refinements` collection: a `union` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `union`, one item of the `refinements` collection.
pub(crate) struct UnionItem;

impl RefinementsItem for UnionItem {
    const TEXT: &'static str = TEXT;
}
