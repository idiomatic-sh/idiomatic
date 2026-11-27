//! Why the lowering refuses a rule. The plan records the refusal's sentence for the rule; the other rules lower.

use super::types::WrittenText;

/// Why a rule does not lower: what of its query, or of a fragment it imports, cannot be read or has no step.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::plan_lowering::errors::PlanLoweringError;
/// use ryo_query_language::domain::plan_lowering::types::WrittenText;
///
/// let unknown = PlanLoweringError::UnknownStep(WrittenText("suppression_matches".into()));
/// assert_eq!(unknown.to_string(), "unknown step suppression_matches");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanLoweringError {
    /// The rule's query could not be read.
    #[error("query not read: {0}")]
    Unread(WrittenText),
    /// A name that is no step the query defined, nor an input of its fragment.
    #[error("unknown step {0}")]
    UnknownStep(WrittenText),
    /// A `$` naming no parameter of the fragment.
    #[error("unknown parameter {0}")]
    UnknownParameter(WrittenText),
    /// An `input` or `param` its caller does not give, or written in a rule, which has no caller.
    #[error("no caller gives {0}")]
    NotGiven(WrittenText),
    /// An import naming no fragment of the pack.
    #[error("no fragment {0}")]
    UnknownFragment(WrittenText),
    /// A relation argument that is neither a relation nor a step.
    #[error("relation argument {0}")]
    RelationArgument(WrittenText),
    /// A select keeps a computed column without naming it with `as`.
    #[error("computed column without a name")]
    UnnamedColumn,
    /// An operator of the step set given an argument it does not take, or of a shape it does not take.
    #[error("operator {operator}: argument {argument}")]
    OperatorArgument {
        /// The operator.
        operator: WrittenText,
        /// The argument.
        argument: WrittenText,
    },
}
