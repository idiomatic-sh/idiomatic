//! The `variadic` item of the `refinements` collection: the C-variadic `...` that ends the parameters of a foreign function.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `variadic`, one item of the `refinements` collection.
pub(crate) struct VariadicItem;

impl RefinementsItem for VariadicItem {
    const TEXT: &'static str = TEXT;
}
