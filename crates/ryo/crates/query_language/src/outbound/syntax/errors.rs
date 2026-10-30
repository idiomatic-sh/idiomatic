//! The `syntax` port's failures.

use ryo_values::domain::program::types::QuerySpan;

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
}
