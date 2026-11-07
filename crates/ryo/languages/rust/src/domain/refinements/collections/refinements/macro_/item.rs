//! The `macro` item of the `refinements` collection: a `macro` definition.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `macro`, one item of the `refinements` collection.
pub(crate) struct MacroItem;

impl RefinementsItem for MacroItem {
    const TEXT: &'static str = TEXT;
}
