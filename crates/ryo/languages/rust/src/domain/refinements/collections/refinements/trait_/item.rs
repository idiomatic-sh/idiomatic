//! The `trait` item of the `refinements` collection: a `trait` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `trait`, one item of the `refinements` collection.
pub(crate) struct TraitItem;

impl RefinementsItem for TraitItem {
    const TEXT: &'static str = TEXT;
}
