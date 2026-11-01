//! The `typecheck` use case's failures.

use ryo_values::domain::program::types::{QueryOrigin, QuerySpan};

/// Why `typecheck` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TypecheckError {
    /// A lower-level refusal attributed to an original module source.
    #[error(
        "{error} (from `{source}` at byte {offset})",
        source = origin.source_id.0,
        offset = origin.span.start.0
    )]
    Located {
        /// The original source and trace.
        origin: QueryOrigin,
        /// The underlying refusal.
        error: Box<TypecheckError>,
    },
    /// The text is not RyoQL.
    #[error("{message} at byte {}", span.start.0)]
    Syntax {
        /// Where the text stops making sense.
        span: QuerySpan,
        /// What was expected there.
        message: String,
    },
    /// The query reads a relation ryo does not own.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The relation's name.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A comparison puts unlike types side by side.
    #[error("{message}")]
    Type {
        /// Where the comparison is.
        span: QuerySpan,
        /// Which types.
        message: String,
    },
    /// The query is wrong in another way: an unknown step, parameter or operation, a missing argument.
    #[error("{message}")]
    Invalid {
        /// Where.
        span: QuerySpan,
        /// What is wrong.
        message: String,
    },
    /// The catalog of relations could not be read.
    #[error("the relation catalog failed: {reason}")]
    Catalog {
        /// What the catalog said.
        reason: String,
    },
}
