//! The `validation` concept's failures.

use ryo_values::domain::query_tokens::types::{QueryOrigin, QuerySpan};

/// Why a program is not valid.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ValidationError {
    /// A named output must have a stable nonblank public identity.
    #[error("an observation output name is blank")]
    BlankObservation { span: QuerySpan },
    /// A step validation failure attributed to its original module source.
    #[error("{error}")]
    Located {
        /// The original source and import/call trace.
        origin: QueryOrigin,
        /// The validation refusal.
        error: Box<ValidationError>,
    },
    /// A query reads a relation ryo does not own.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The relation's name.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A query reads a step that was not defined above it.
    #[error("no step named `{name}` above this line")]
    UnknownStep {
        /// The step's name.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A query reads a parameter it did not declare.
    #[error("no parameter named `{name}`")]
    UnknownParameter {
        /// The parameter's name.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A query calls an operation the language does not have.
    #[error("no operation named `{name}`")]
    UnknownOperation {
        /// The operation's name.
        name: String,
        /// Where the query calls it.
        span: QuerySpan,
    },
    /// A call leaves out an argument its operation requires.
    #[error("`{operation}` needs the argument `{argument}`")]
    MissingArgument {
        /// The operation as written.
        operation: String,
        /// The argument as written.
        argument: String,
        /// Where the call is.
        span: QuerySpan,
    },
    /// Two explicit exports have one public name.
    #[error("the observation `{name}` is exported twice")]
    DuplicateObservation {
        /// The repeated public output name.
        name: String,
        /// The second declaration.
        span: QuerySpan,
    },
    /// Two steps have one name.
    #[error("the step `{name}` is defined twice")]
    DuplicateStep {
        /// The step's name.
        name: String,
        /// Where it is defined the second time.
        span: QuerySpan,
    },
    /// A `select` with a `join` does not say which columns it keeps.
    #[error("a select with a join must name the columns it keeps")]
    JoinNeedsColumns {
        /// Where the select is.
        span: QuerySpan,
    },
}
