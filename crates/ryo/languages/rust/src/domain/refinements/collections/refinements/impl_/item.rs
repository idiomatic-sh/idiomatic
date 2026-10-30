//! The `impl` item of the `refinements` collection: an `impl` block.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `impl`, one item of the `refinements` collection.
pub(crate) struct ImplItem;

impl RefinementsItem for ImplItem {
    const TEXT: &'static str = TEXT;
}
