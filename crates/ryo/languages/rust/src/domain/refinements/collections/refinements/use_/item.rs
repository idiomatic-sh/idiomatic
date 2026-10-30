//! The `use` item of the `refinements` collection: a name a `use` brings in.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `use`, one item of the `refinements` collection.
pub(crate) struct UseItem;

impl RefinementsItem for UseItem {
    const TEXT: &'static str = TEXT;
}
