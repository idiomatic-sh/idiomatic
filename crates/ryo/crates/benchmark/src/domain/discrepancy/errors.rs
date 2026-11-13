//! Why a reference discrepancy is refused.

/// A refused reference discrepancy.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DiscrepancyError {
    /// Refuses: an identity is not a lowercase SHA256.
    #[error("discrepancy requires lowercase SHA256 identities")]
    IdentityNotLowercaseSha256,
    /// Refuses: a process expectation was asked for diagnostics.
    #[error("process expectation is not a diagnostic report")]
    ProcessExpectationNotDiagnostic,
    /// Refuses: execution accounting is incomplete or unknown.
    #[error("reference discrepancy requires complete, known execution coverage")]
    CoverageNotCompleteAndKnown,
    /// Refuses: the tool, profile or executable is not the pinned one.
    #[error("stale reference discrepancy tool/profile or unapproved executable")]
    StaleToolProfileOrExecutable,
    /// Refuses: the expectations are not distinct, full and evidenced.
    #[error("reference discrepancy requires distinct full expectations and independent evidence")]
    ExpectationsNotDistinctFullEvidenced,
    /// Refuses: the language evidence does not support the configured contract.
    #[error("independent evidence does not support configured language contract")]
    EvidenceDoesNotSupportConfiguration,
    /// Refuses: the authority and the evidence kind disagree.
    #[error("discrepancy authority and evidence differ")]
    AuthorityAndEvidenceDiffer,
}

impl From<DiscrepancyError> for ryo_benchmark_contracts::domain::reports::errors::ReportsError {
    fn from(error: DiscrepancyError) -> Self {
        Self::Invalid(error.to_string())
    }
}
