//! The `placement` concept's failures.

use crate::domain::selectors::errors::SelectorsError;

/// Why a workspace could not be placed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlacementError {
    /// An entry's paths do not resolve.
    #[error("entry `{entry}`: {source}")]
    Selector {
        /// The entry.
        entry: String,
        /// Why.
        source: SelectorsError,
    },
    /// An entry names a release the workspace did not load.
    #[error("entry `{entry}`: {release} was not loaded")]
    Unloaded {
        /// The entry.
        entry: String,
        /// The release.
        release: String,
    },
}

impl From<crate::domain::template_placing::errors::TemplatePlacingError> for PlacementError {
    fn from(error: crate::domain::template_placing::errors::TemplatePlacingError) -> Self {
        match error {
            crate::domain::template_placing::errors::TemplatePlacingError::Selector {
                entry,
                source,
            } => Self::Selector { entry, source },
        }
    }
}
