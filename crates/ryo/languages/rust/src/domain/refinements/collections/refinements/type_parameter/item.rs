//! The `type-parameter` item of the `refinements` collection: a generic type parameter.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `type-parameter`, one item of the `refinements` collection.
pub(crate) struct TypeParameterItem;

impl RefinementsItem for TypeParameterItem {
    const TEXT: &'static str = TEXT;
}
