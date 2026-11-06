//! Why a receipt is refused.
use crate::domain::registered_inputs::{errors::RegisteredInputsError, types::RefusalText};

/// A refused receipt: each refusal of the registered and native documents is its own variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InventoryReceiptsError {
    /// Refuses: diagnostic span or identity is invalid.
    #[error("diagnostic span or identity is invalid")]
    DiagnosticSpanOrIdentityInvalid,
    /// Refuses: missing or extra native case receipts.
    #[error("missing or extra native case receipts")]
    NativeCaseReceiptCount,
    /// Refuses: registered rule identity is missing.
    #[error("registered rule identity is missing")]
    RegisteredRuleMissing,
    /// Refuses: registered case identity is missing.
    #[error("registered case identity is missing")]
    RegisteredCaseMissing,
    /// Refuses: duplicate registered case identity.
    #[error("duplicate registered case identity")]
    DuplicateRegisteredCase,
    /// Refuses: native rule identity is missing.
    #[error("native rule identity is missing")]
    NativeRuleMissing,
    /// Refuses: native case identity is missing.
    #[error("native case identity is missing")]
    NativeCaseMissing,
    /// Refuses: native side identity is missing.
    #[error("native side identity is missing")]
    NativeSideMissing,
    /// Refuses: unregistered native case observation.
    #[error("unregistered native case observation")]
    UnregisteredObservation,
    /// Refuses: duplicate native case observation.
    #[error("duplicate native case observation")]
    DuplicateObservation,
    /// Refuses: registered file metadata digest differs.
    #[error("registered file metadata digest differs")]
    FileMetadataDigestDiffers,
    /// Refuses: registered project inputs digest differs.
    #[error("registered project inputs digest differs")]
    ProjectInputsDigestDiffers,
    /// Refuses: process expectation cannot satisfy complete-report accounting.
    #[error("process expectation cannot satisfy complete-report accounting")]
    ProcessExpectationNotComplete,
    /// Refuses: discrepancy intention differs from registered case.
    #[error("discrepancy intention differs from registered case")]
    DiscrepancyIntentionDiffers,
    /// Refuses: missing or duplicate native side receipt.
    #[error("missing or duplicate native side receipt")]
    SideReceiptCount,
    /// Refuses: native configured rule context differs from registration.
    #[error("native configured rule context differs from registration")]
    RuleContextDiffers,
    /// Refuses: native observation declarations differ from registration.
    #[error("native observation declarations differ from registration")]
    ObservationDeclarationsDiffer,
    /// Refuses: native selected-rule cohort differs from registration.
    #[error("native selected-rule cohort differs from registration")]
    SelectedCohortDiffers,
    /// Refuses: native expectation contract differs from registration.
    #[error("native expectation contract differs from registration")]
    ExpectationContractDiffers,
    /// Refuses: failed, incomplete, or stale native observation.
    #[error("failed, incomplete, or stale native observation")]
    ObservationStale,
    /// Refuses: native discrepancy authority differs from registration.
    #[error("native discrepancy authority differs from registration")]
    DiscrepancyAuthorityDiffers,
    /// Refuses: diagnostic report unexpectedly contains a process failure.
    #[error("diagnostic report unexpectedly contains a process failure")]
    UnexpectedProcessObservation,
    /// Refuses: native diagnostics differ from registered expectations.
    #[error("native diagnostics differ from registered expectations")]
    DiagnosticsDiffer,
    /// Refuses: candidate execution is not completely accounted.
    #[error("candidate execution is not completely accounted")]
    CandidateNotAccounted,
    /// Refuses: unexpected project-context receipt.
    #[error("unexpected project-context receipt")]
    UnexpectedProjectContext,
    /// Refuses: stale project-context receipt.
    #[error("stale project-context receipt")]
    StaleProjectContext,
    /// Refuses: discrepancy file metadata identity differs.
    #[error("discrepancy file metadata identity differs")]
    DiscrepancyMetadataDiffers,
    /// Refuses: missing independent discrepancy validation.
    #[error("missing independent discrepancy validation")]
    DiscrepancyValidationMissing,
    /// Refuses: discrepancy authority was not verified.
    #[error("discrepancy authority was not verified")]
    DiscrepancyAuthorityUnverified,
    /// Refuses: stale discrepancy evidence identity.
    #[error("stale discrepancy evidence identity")]
    StaleDiscrepancyEvidence,
    /// Refuses: discrepancy tool identity is missing.
    #[error("discrepancy tool identity is missing")]
    DiscrepancyToolMissing,
    /// Refuses: discrepancy tool is not independently pinned.
    #[error("discrepancy tool is not independently pinned")]
    DiscrepancyToolUnpinned,
    /// Refuses: diagnostics are missing.
    #[error("diagnostics are missing")]
    DiagnosticsMissing,
    /// The registered case's input identities are refused.
    #[error(transparent)]
    Inputs(#[from] RegisteredInputsError),
    /// A not yet typed neighbour refused with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}

impl From<String> for InventoryReceiptsError {
    fn from(text: String) -> Self {
        Self::Refused(RefusalText(text))
    }
}

impl From<InventoryReceiptsError> for String {
    fn from(error: InventoryReceiptsError) -> Self {
        error.to_string()
    }
}
