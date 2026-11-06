//! The `query_language` port's failures.

use ryo_values::domain::program::types::RefusedQuery;

/// Why the `query_language` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueryLanguageError {
    /// The query text is wrong; the error says where and how.
    #[error("{error}")]
    Refused {
        /// Why the text was refused, and where.
        error: RefusedQuery,
    },
}
