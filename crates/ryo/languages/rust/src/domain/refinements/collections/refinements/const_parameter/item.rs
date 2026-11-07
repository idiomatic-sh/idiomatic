//! The `const-parameter` item of the `refinements` collection: a const generic parameter.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `const-parameter`, one item of the `refinements` collection.
pub(crate) struct ConstParameterItem;

impl RefinementsItem for ConstParameterItem {
    const TEXT: &'static str = TEXT;
}
