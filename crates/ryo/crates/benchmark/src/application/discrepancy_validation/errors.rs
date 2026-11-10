//! Why a reference discrepancy cannot be validated against its evidence.
use crate::domain::discrepancy::errors::DiscrepancyError;
use crate::domain::invocation_identity::errors::InvocationIdentityError;
use crate::domain::process_failure::errors::ProcessFailureError;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// A refused reference discrepancy.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DiscrepancyValidationError {
    /// Refuses: the rules directory names no profile root.
    #[error("missing profile root")]
    MissingProfileRoot,
    /// Refuses: the certificate identities are stale.
    #[error("stale reference discrepancy settings/source/expectation identity")]
    StaleIdentity,
    /// Refuses: archived bytes, termination or identity differ.
    #[error("archived process bytes, termination or immutable identity differ")]
    ArchivedBytesDiffer,
    /// Refuses: archived argv and environment do not establish the pinned settings.
    #[error("archived argv/environment do not establish the pinned harness settings")]
    ArchivedSettingsNotEstablished,
    /// Refuses: the discrepancy rules refused it.
    #[error(transparent)]
    Discrepancy(#[from] DiscrepancyError),
    /// Refuses: the archived process failure is not reproducible.
    #[error(transparent)]
    Archive(#[from] ProcessFailureError),
    /// Refuses: the invocation cannot be normalized.
    #[error(transparent)]
    Invocation(#[from] InvocationIdentityError),
    /// A library or a boundary refused with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}

impl From<ReportsError> for DiscrepancyValidationError {
    fn from(error: ReportsError) -> Self {
        Self::Refused(RefusalText(error.to_string()))
    }
}

impl From<ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError>
    for DiscrepancyValidationError
{
    fn from(error: ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError) -> Self {
        Self::Refused(RefusalText(error.to_string()))
    }
}

impl From<DiscrepancyValidationError> for ReportsError {
    fn from(error: DiscrepancyValidationError) -> Self {
        Self::Invalid(error.to_string())
    }
}
