//! What every selector kind implements: it resolves the selectors of its kind at one place.

use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::types::{Resolution, SelectionContext, Selector};

/// One selector kind.
pub(crate) trait SelectorsItem: Sync {
    /// The selector resolved at the place `context` describes, or `NotMine` when it is of another kind.
    ///
    /// # Errors
    ///
    /// `Pattern` when its pattern does not compile.
    fn resolve(
        &self,
        selector: &Selector,
        context: &SelectionContext<'_>,
    ) -> Result<Resolution, SelectorsError>;
}
