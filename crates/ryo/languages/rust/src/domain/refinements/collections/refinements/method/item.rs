//! The `method` item of the `refinements` collection: a function of an `impl` or a `trait` with a `self` receiver.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `method`, one item of the `refinements` collection.
pub(crate) struct MethodItem;

impl RefinementsItem for MethodItem {
    const TEXT: &'static str = TEXT;
}
