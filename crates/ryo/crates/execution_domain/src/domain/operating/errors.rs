//! The `operating` concept's failures.

use ryo_language::domain::semantics::errors::SemanticsError;
use ryo_values::domain::rows::errors::RowsError;

/// Why an operator could not compute its rows.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OperatingError {
    /// A spelling check failed.
    #[error(transparent)]
    Spelling(#[from] crate::domain::spelling::errors::SpellingError),
    /// Request declaration errors are preserved before any file is read.
    #[error(transparent)]
    ObservationConfiguration(#[from] crate::domain::rule_intent::errors::RuleIntentError),
    /// An explicit bound producer did not publish a complete result in this generation.
    #[error("named producer is incomplete: {reason}")]
    ObservationIncomplete {
        /// Why the producer's result is incomplete.
        reason: String,
    },
    /// A semantic request named a stale or foreign syntax node.
    #[error(transparent)]
    SemanticRender(#[from] SemanticsError),
    /// The provider returned a different number of answers than requested.
    #[error("semantic renderer returned a malformed batch")]
    SemanticBatchShape,
    /// A valid semantic request cannot be answered completely by this provider.
    #[error("semantic rendering is incomplete: {reason}")]
    SemanticIncomplete {
        /// The application's or provider's exact reason.
        reason: String,
    },
    /// Ordered DFS cannot distinguish two outgoing edges of one partition/source.
    #[error("ordered-dfs requires unique order values within each partition and source")]
    DuplicateDfsOrder,
    /// A visit ordinal cannot be represented as an engine integer.
    #[error("ordered-dfs visit ordinal overflow")]
    DfsOrdinalOverflow,
    /// An input or composed span falls outside the current file.
    #[error("span-between encountered an invalid byte range")]
    InvalidSpan,
    /// A source match cannot be located within valid UTF-8 and physical lines.
    #[error("source-matches requires an in-file UTF-8 span and valid physical line")]
    InvalidSourceMatchSpan,
    /// A span has no representable ordered endpoints.
    #[error("span-bounds requires a present, ordered span")]
    InvalidSpanBounds,
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
    Table(#[from] RowsError),
}

impl From<ryo_operators::domain::comparing::errors::ComparingError> for OperatingError {
    fn from(fault: ryo_operators::domain::comparing::errors::ComparingError) -> Self {
        use ryo_operators::domain::comparing::errors::ComparingError as Fault;
        match fault {
            Fault::WrongTest => Self::WrongTest,
            Fault::Unbound => Self::Unbound,
            Fault::WrongBinding => Self::WrongBinding,
            Fault::InvalidRegex { reason } => Self::InvalidRegex { reason },
            Fault::Rows(error) => Self::Table(error),
        }
    }
}

impl From<ryo_operators::domain::patterns::errors::PatternsError> for OperatingError {
    fn from(fault: ryo_operators::domain::patterns::errors::PatternsError) -> Self {
        use ryo_operators::domain::patterns::errors::PatternsError as Fault;
        match fault {
            Fault::InvalidRegex { reason } => Self::InvalidRegex { reason },
        }
    }
}
