//! The `receiver` item of the `refinements` collection: the value a method is called on.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `receiver`, one item of the `refinements` collection.
pub(crate) struct ReceiverItem;

impl RefinementsItem for ReceiverItem {
    const TEXT: &'static str = TEXT;
}
