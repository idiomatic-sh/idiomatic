//! The `lifetime-parameter` item of the `refinements` collection: a lifetime parameter.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `lifetime-parameter`, one item of the `refinements` collection.
pub(crate) struct LifetimeParameterItem;

impl RefinementsItem for LifetimeParameterItem {
    const TEXT: &'static str = TEXT;
}
