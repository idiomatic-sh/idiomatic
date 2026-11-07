//! Why two native observations cannot be compared.
use crate::domain::registered_inputs::types::RefusalText;

/// A refusal to compare two native observations: each missing or contradictory piece of evidence is its own variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InventoryComparisonError {
    /// Refuses: an observation has no execution capability.
    #[error("missing or malformed execution capability")]
    MissingExecutionCapability,
    /// Refuses: the execution capability is unknown or unavailable without a reason.
    #[error("execution capability is not explicitly declared")]
    UndeclaredExecutionCapability,
    /// Refuses: an observed file path is empty.
    #[error("invalid execution file")]
    InvalidExecutionFile,
    /// Refuses: an observation records no skip list.
    #[error("missing execution skips")]
    MissingExecutionSkips,
    /// Refuses: a skipped rule identity is invalid.
    #[error("invalid skipped rule")]
    InvalidSkippedRule,
    /// Refuses: an unavailable capability asserts accounting or skips.
    #[error("unavailable execution capability contains asserted accounting")]
    UnavailableCapabilityAsserted,
    /// Refuses: a process failure is compared under a contract other than diagnostics.
    #[error("process failure has no complete-report execution accounting")]
    ProcessFailureHasNoCompleteReport,
    /// Refuses: the two reports do not satisfy the declared contract.
    #[error("execution evidence does not satisfy the declared contract")]
    ExecutionEvidenceUnsatisfied,
    /// The candidate's execution is not complete, with the words of the refusal.
    #[error("{}", .0.0)]
    CandidateExecutionIncomplete(RefusalText),
}
