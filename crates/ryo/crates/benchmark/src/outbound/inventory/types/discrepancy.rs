//! A documented difference from the reference tool, and the validation a native side attests.
use super::finding::Finding;
use super::text::{PathText, ReasonText, Sha256Text};
use crate::outbound::suite::types::process::ProcessFailureWire;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;

/// The sealed identities a discrepancy is bound to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscrepancyIdentity {
    /// The reference tool's name.
    pub tool: Option<ReasonText>,
    /// The reference tool's version.
    pub version: Option<ReasonText>,
    /// The executable bytes the evidence was observed with.
    pub observed_executable_sha256: Option<Sha256Text>,
    /// The pinned profile.
    pub profile: Option<Sha256Text>,
    /// The normalized invocation settings.
    pub settings: Option<Sha256Text>,
    /// The case sources.
    pub source: Option<Sha256Text>,
    /// The reference output.
    pub reference_output: Option<Sha256Text>,
    /// The intended output.
    pub intended_output: Option<Sha256Text>,
    /// The declared file metadata.
    pub file_metadata: Option<Sha256Text>,
}

/// A documented difference between the reference tool and the intended behavior.
#[derive(Clone, Debug)]
pub struct ReferenceDiscrepancy {
    /// The authority the difference rests on.
    pub basis: DiscrepancyBasis,
    /// The identities the evidence is bound to.
    pub identity: DiscrepancyIdentity,
    /// The executables the evidence may be observed with.
    pub approved_executable_sha256: Vec<Sha256Text>,
    /// What the reference reports, absent for a process failure.
    pub reference_expected: Option<Vec<Finding>>,
    /// What the intended behavior reports.
    pub intended_expected: Vec<Finding>,
    /// Why the difference is accepted.
    pub reason: Option<ReasonText>,
    /// The explicit process failure, when the reference does not report diagnostics.
    pub process: Option<ProcessFailureWire>,
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscrepancyValidation {
    /// The authority validated.
    pub discrepancy_basis: Option<DiscrepancyBasis>,
    /// Digest of the process observation group.
    pub process_observation_group_sha256: Option<Sha256Text>,
    /// The reference expectation kind.
    pub reference_expectation_kind: Option<ExpectationKind>,
    /// The normalized invocation settings.
    pub settings_sha256: Option<Sha256Text>,
    /// The executable that ran.
    pub actual_executable_sha256: Option<Sha256Text>,
    /// The pinned profile.
    pub profile_sha256: Option<Sha256Text>,
    /// The case sources.
    pub source_sha256: Option<Sha256Text>,
    /// The reference output.
    pub reference_output_sha256: Option<Sha256Text>,
    /// The intended output.
    pub intended_output_sha256: Option<Sha256Text>,
    /// The declared file metadata.
    pub file_metadata_sha256: Option<Sha256Text>,
}

/// A raw process artifact an observation points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPointer {
    /// The receipt-relative path.
    pub path: PathText,
    /// The artifact's byte digest.
    pub sha256: Sha256Text,
}
