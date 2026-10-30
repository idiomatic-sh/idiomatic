//! The `positional` item of the `refinements` collection: a parameter passed by position, written out.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `positional`, one item of the `refinements` collection.
pub(crate) struct PositionalItem;

impl RefinementsItem for PositionalItem {
    const TEXT: &'static str = TEXT;
}
