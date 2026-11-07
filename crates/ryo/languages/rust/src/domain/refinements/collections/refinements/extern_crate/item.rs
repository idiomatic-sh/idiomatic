//! The `extern-crate` item of the `refinements` collection: a name an `extern crate` brings in.

use super::super::interface::RefinementsItem;
use super::constants::TEXT;

/// `extern-crate`, one item of the `refinements` collection.
pub(crate) struct ExternCrateItem;

impl RefinementsItem for ExternCrateItem {
    const TEXT: &'static str = TEXT;
}
