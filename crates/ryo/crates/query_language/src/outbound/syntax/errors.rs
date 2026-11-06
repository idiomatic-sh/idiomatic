//! The `syntax` port's failures.

use ryo_values::domain::query_tokens::types::{QueryOrigin, QuerySpan};

/// Why the `syntax` port failed.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SyntaxError {
    /// The text is not RyoQL.
    #[error("{message} at byte {}", span.start.0)]
    Unreadable {
        /// Where the text stops making sense.
        span: QuerySpan,
        /// What was expected there.
        message: String,
    },
    /// A fragment source failed while parsing or linking, with its original bytes and trace.
    #[error("{message} in query source `{source_id}` at byte {offset}", source_id = origin.source_id.0, offset = origin.span.start.0)]
    Located {
        /// The original source and import/call path.
        origin: QueryOrigin,
        /// What failed.
        message: String,
    },
}
