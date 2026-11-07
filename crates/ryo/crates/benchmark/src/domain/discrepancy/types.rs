//! Data of the reference discrepancy rules.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;

/// A lowercase SHA256 identity as a certificate spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IdentityText(pub String);

impl From<String> for IdentityText {
    fn from(text: String) -> Self {
        Self(text)
    }
}

/// Which side of the comparison a report belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contestant {
    /// The pinned reference tool.
    Reference,
    /// The tool under measurement.
    Candidate,
}

/// Whether a report agrees with the intended contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntentAgreement {
    /// The report is exactly what the intended contract asks.
    Matches,
    /// The report differs from the intended contract.
    Differs,
}

/// A digest that a proof may leave out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OptionalDigest {
    /// The proof binds no such identity.
    Absent,
    /// The proof binds this identity.
    Pinned(ArtifactDigest),
}

/// What kind of expectation the reference side was validated against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReferenceKind {
    /// Complete diagnostics.
    Diagnostics,
    /// An explicit process failure.
    ProcessFailure,
}

/// Everything a validated reference discrepancy binds an observation to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReferenceProof {
    /// The authority the difference rests on.
    pub basis: DiscrepancyBasis,
    /// The digest of the chosen exact-binary archive group, for a process failure.
    pub process_group: OptionalDigest,
    /// What the reference was validated against.
    pub kind: ReferenceKind,
    /// The normalized invocation settings.
    pub settings: ArtifactDigest,
    /// The executable that ran.
    pub executable: ArtifactDigest,
    /// The pinned profile.
    pub profile: ArtifactDigest,
    /// The case sources.
    pub source: ArtifactDigest,
    /// The reference output.
    pub reference_output: ArtifactDigest,
    /// The intended output.
    pub intended_output: ArtifactDigest,
    /// The declared file metadata.
    pub file_metadata: OptionalDigest,
}

impl std::fmt::Display for Contestant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Reference => super::constants::REFERENCE,
            Self::Candidate => super::constants::CANDIDATE,
        })
    }
}

/// What comparing one contestant's report with its expectation showed.
pub(crate) struct Judgement {
    /// Nothing, or both raw finding multisets and skip arrays of the difference.
    pub comparison: Result<(), ryo_benchmark_contracts::domain::reports::types::Difference>,
}
