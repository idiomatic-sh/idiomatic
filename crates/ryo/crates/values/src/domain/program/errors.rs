//! Refusals when building an immutable query source bundle.

use crate::domain::query_tokens::types::QuerySourceId;

/// A query bundle could not be constructed without ambiguity.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProgramError {
    /// A source identity is blank.
    #[error("query source id is blank")]
    BlankSourceId,
    /// The root query text is blank.
    #[error("root query text is blank")]
    BlankRootText,
    /// Two sources use the same identity, or a module duplicates the root identity.
    #[error("query source id `{}` is duplicated", .id.0)]
    DuplicateSourceId {
        /// The repeated identity.
        id: QuerySourceId,
    },
}
