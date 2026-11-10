//! Why a reference expectation cannot be attached to its case.
use crate::domain::discrepancy::errors::DiscrepancyError;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// A refused reference expectation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DiscrepancyAttachmentError {
    /// Refuses: the rules directory names no case profile.
    #[error("missing case profile boundary")]
    MissingCaseProfileBoundary,
    /// Refuses: two expectations name one case.
    #[error("duplicate reference expectation identity")]
    DuplicateExpectationIdentity,
    /// Refuses: an expectation names no executable case.
    #[error("reference expectation outside executable case inventory")]
    ExpectationOutsideInventory,
    /// Refuses: a case already carries a canonical expectation.
    #[error("duplicate canonical reference expectation")]
    DuplicateCanonicalExpectation,
    /// Refuses: a process expectation also declares diagnostics.
    #[error("process expectation cannot also declare reference diagnostics")]
    ProcessAlsoDeclaresDiagnostics,
    /// Refuses: a reference diagnostic names no requested file or an inverted range.
    #[error("invalid reference expectation source/range")]
    InvalidSourceRange,
    /// Refuses: a tool-contract difference declares a process failure.
    #[error("tool-contract differences require complete diagnostic outputs")]
    ToolContractNeedsDiagnostics,
    /// Refuses: language evidence names no language contract or rationale.
    #[error("independent evidence requires explicit language contract and version rationale")]
    LanguageEvidenceNeedsContract,
    /// Refuses: tool contract evidence names no pinned source revision or reason.
    #[error("tool contract evidence needs a pinned public source revision and policy reason")]
    ToolContractEvidenceNeedsSource,
    /// Refuses: prior worker evidence names no control or reason.
    #[error("prior worker evidence needs a named control and policy reason")]
    PriorWorkerEvidenceNeedsControl,
    /// Refuses: an evidence row does not match the declared authority.
    #[error("discrepancy evidence does not match its declared authority")]
    EvidenceDoesNotMatchAuthority,
    /// Refuses: a tool contract difference lacks exactly its two evidence rows.
    #[error(
        "tool-contract difference requires one public contract and one prior worker observation"
    )]
    ToolContractNeedsTwoEvidenceRows,
    /// Refuses: an identity is not a lowercase SHA256.
    #[error(transparent)]
    Identity(#[from] DiscrepancyError),
    /// A library or a boundary refused with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}

impl From<ReportsError> for DiscrepancyAttachmentError {
    fn from(error: ReportsError) -> Self {
        Self::Refused(RefusalText(error.to_string()))
    }
}

impl From<ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError>
    for DiscrepancyAttachmentError
{
    fn from(error: ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError) -> Self {
        Self::Refused(RefusalText(error.to_string()))
    }
}

impl From<DiscrepancyAttachmentError> for ReportsError {
    fn from(error: DiscrepancyAttachmentError) -> Self {
        Self::Invalid(error.to_string())
    }
}
