//! The `operating` concept's failures.

use crate::domain::table_model::errors::TableModelError;

/// Why an operator could not compute its rows.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum OperatingError {
    /// Ordered DFS cannot distinguish two outgoing edges of one partition/source.
    #[error("ordered-dfs requires unique order values within each partition and source")]
    DuplicateDfsOrder,
    /// A visit ordinal cannot be represented as an engine integer.
    #[error("ordered-dfs visit ordinal overflow")]
    DfsOrdinalOverflow,
    /// An input or composed span falls outside the current file.
    #[error("span-between encountered an invalid byte range")]
    InvalidSpan,
    /// Ordering cannot distinguish two rows in the same key group.
    #[error("ordered-match requires unique order values within each side and key group")]
    DuplicateOrder,
    /// A step has fewer inputs than its operator reads.
    #[error("the operator needs input {position}")]
    MissingInput {
        /// Which input, counting from one.
        position: usize,
    },
    /// An operator was given an operation that is not its own.
    #[error("the operation is not this operator's")]
    WrongOperation,
    /// A step reads a step that has not run.
    #[error("a step reads a step that has not run")]
    NotRun,
    /// A step reads a relation where only a step's rows can be read.
    #[error("a step reads a relation where it can read only a step")]
    NotAStep,
    /// A step reads a relation nobody read.
    #[error("a step reads a relation that was not read")]
    NotRead,
    /// A query reads a parameter nothing was bound to.
    #[error("the parameter has no value")]
    Unbound,
    /// A parameter was bound to a value of the wrong kind.
    #[error("the parameter is bound to the wrong kind of value")]
    WrongBinding,
    /// A comparison family was given a test that is not its own.
    #[error("the family does not have this test")]
    WrongTest,
    /// A query supplied an invalid regular expression.
    #[error("invalid regular expression: {reason}")]
    InvalidRegex {
        /// The regular expression compiler's explanation.
        reason: String,
    },
    /// Two inputs that should have the same columns do not.
    #[error("the inputs have different columns")]
    ShapeMismatch,
    /// A step reads a column its input lacks.
    #[error(transparent)]
    Table(#[from] TableModelError),
}
