//! A documented difference from the reference tool, and the validation a native side attests.
use crate::domain::documents::types::finding::value::{ExpectedFindings, RecordedFinding};
use crate::domain::documents::types::text::value::{FileMetadataDigest, PathText, Pin, ReasonText, Sha256Text};
use crate::domain::process::types::ProcessFailure;
use crate::domain::workloads::types::DiscrepancyBasis;

/// The executable the evidence was observed with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservedExecutable {
    /// Written for a language defect or a tool contract difference.
    Observed(Sha256Text),
    /// Absent for an explicit process failure.
    NotObserved,
}

/// The sealed identities a discrepancy is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscrepancyIdentity {
    /// The reference tool's name.
    pub tool: ReasonText,
    /// The reference tool's version.
    pub version: ReasonText,
    /// The executable bytes the evidence was observed with.
    pub observed_executable_sha256: ObservedExecutable,
    /// The pinned profile.
    pub profile: Sha256Text,
    /// The normalized invocation settings.
    pub settings: Sha256Text,
    /// The case sources.
    pub source: Sha256Text,
    /// The reference output.
    pub reference_output: Sha256Text,
    /// The intended output.
    pub intended_output: Sha256Text,
    /// The declared file metadata: written when the case declares file metadata.
    pub file_metadata: FileMetadataDigest,
}

/// A documented difference between the reference tool and the intended behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscrepancyRecord {
    /// The authority the difference rests on; a process failure leaves it to its default.
    pub basis: DiscrepancyBasis,
    /// The identities the evidence is bound to.
    pub identity: DiscrepancyIdentity,
    /// The executables the evidence may be observed with.
    pub approved_executable_sha256: Vec<Sha256Text>,
    /// What the reference reports.
    pub reference_expected: ExpectedFindings,
    /// What the intended behavior reports.
    pub intended_expected: Vec<RecordedFinding>,
    /// Why the difference is accepted.
    pub reason: ReasonText,
    /// The explicit process failure: present when the reference does not report diagnostics.
    pub process: ProcessExpectation,
}

/// Whether the reference is expected to fail as a process instead of reporting diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessExpectation {
    /// The reference reports diagnostics.
    Absent,
    /// The reference fails with this explicit process failure.
    Failure(ProcessFailure),
}

/// What kind of expectation the reference side was validated against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectationKind {
    /// Diagnostics.
    Diagnostics,
    /// A process failure.
    ProcessFailure,
}

/// What a native side attests it validated for a discrepancy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscrepancyValidation {
    /// The authority validated.
    pub discrepancy_basis: DiscrepancyBasis,
    /// Digest of the process observation group: written for an explicit process failure.
    pub process_observation_group_sha256: Pin,
    /// The reference expectation kind.
    pub reference_expectation_kind: ExpectationKind,
    /// The normalized invocation settings.
    pub settings_sha256: Sha256Text,
    /// The executable that ran.
    pub actual_executable_sha256: Sha256Text,
    /// The pinned profile.
    pub profile_sha256: Sha256Text,
    /// The case sources.
    pub source_sha256: Sha256Text,
    /// The reference output.
    pub reference_output_sha256: Sha256Text,
    /// The intended output.
    pub intended_output_sha256: Sha256Text,
    /// The declared file metadata: written when the case declares file metadata.
    pub file_metadata_sha256: FileMetadataDigest,
}

/// A raw process artifact an observation points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPointer {
    /// The receipt-relative path.
    pub path: PathText,
    /// The artifact's byte digest.
    pub sha256: Sha256Text,
}
