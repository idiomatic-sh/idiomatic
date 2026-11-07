//! The `type-alias` item of the `refinements` collection: a `type` alias.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `type-alias`, one item of the `refinements` collection.
pub(crate) struct TypeAliasItem;

impl RefinementsItem for TypeAliasItem {
    const TEXT: &'static str = TEXT;
}
