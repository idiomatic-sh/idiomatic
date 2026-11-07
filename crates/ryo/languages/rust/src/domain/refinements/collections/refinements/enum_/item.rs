//! The `enum` item of the `refinements` collection: an `enum` item.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `enum`, one item of the `refinements` collection.
pub(crate) struct EnumItem;

impl RefinementsItem for EnumItem {
    const TEXT: &'static str = TEXT;
}
