//! The `module` item of the `refinements` collection: a `mod` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `module`, one item of the `refinements` collection.
pub(crate) struct ModuleItem;

impl RefinementsItem for ModuleItem {
    const TEXT: &'static str = TEXT;
}
