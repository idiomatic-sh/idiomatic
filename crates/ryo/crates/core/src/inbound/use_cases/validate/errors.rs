//! The `validate` use case's failures.

use ryo_values::domain::plan_parts::types::RefusedQuery;

/// Why `validate` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidateError {
    /// The text is not a correct query: not RyoQL, a relation neither ryo nor the host has, unlike types, or wrong in another way.
    #[error("{error}")]
    Refused {
        /// Why the text was refused, and where.
        error: RefusedQuery,
    },
    /// The query is correct but cannot be compiled to run.
    #[error("{error}")]
    Uncompilable {
        /// Why the compiler refused it.
        error: RefusedQuery,
    },
    /// The query compares the kind of a node or a token with a word that names no kind of any linked language.
    #[error("{reason}")]
    UnknownKind {
        /// The word and the languages it names no kind of.
        reason: String,
    },
}
