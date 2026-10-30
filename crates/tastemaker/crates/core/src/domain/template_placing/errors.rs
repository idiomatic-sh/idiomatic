//! The `template_placing` concept's failures.

use crate::domain::selectors::errors::SelectorsError;

/// Why a template entry's files could not be placed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TemplatePlacingError {
    /// An entry's file pattern or its `exclude` does not compile.
    #[error("entry `{entry}`: {source}")]
    Selector {
        /// The entry.
        entry: String,
        /// Why.
        source: SelectorsError,
    },
}
