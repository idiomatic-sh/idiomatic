//! Refusals of a measured run.
/// A measured run carries no declared file observations: verification owns them.
pub(super) const DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT: &str =
    "declared file observations require verification preparation";
/// Refusal of a candidate run that collected no CPU or peak.
pub(super) const NO_RESOURCES: &str = "the candidate's run collected no CPU time or peak";
/// Separator between explicit per-rule finding counts.
pub(super) const FINDING_SEPARATOR: &str = ", ";
/// Increment for one diagnostic occurrence.
pub(super) const ONE_FINDING: usize = 1;
/// Stable observation label for reference-first execution.
pub(super) const REFERENCE_FIRST: &str = "ReferenceFirst";
/// Stable observation label for candidate-first execution.
pub(super) const CANDIDATE_FIRST: &str = "CandidateFirst";
