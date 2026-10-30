//! The `macro-rules` item of the `refinements` collection: a `macro_rules!` definition.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `macro-rules`, one item of the `refinements` collection.
pub(crate) struct MacroRulesItem;

impl RefinementsItem for MacroRulesItem {
    const TEXT: &'static str = TEXT;
}
