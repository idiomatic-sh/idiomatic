//! Refusals when building an immutable query source bundle.

use super::types::QuerySourceId;

/// A query bundle could not be constructed without ambiguity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryBundleError {
    /// A source identity is blank.
    BlankSourceId,
    /// The root query text is blank.
    BlankRootText,
    /// Two sources use the same identity, or a module duplicates the root identity.
    DuplicateSourceId {
        /// The repeated identity.
        id: QuerySourceId,
    },
}

impl std::fmt::Display for QueryBundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankSourceId => f.write_str("query source id is blank"),
            Self::BlankRootText => f.write_str("root query text is blank"),
            Self::DuplicateSourceId { id } => {
                write!(f, "query source id `{}` is duplicated", id.0)
            }
        }
    }
}

impl std::error::Error for QueryBundleError {}
