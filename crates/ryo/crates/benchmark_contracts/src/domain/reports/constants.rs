//! Exact refusal vocabulary for shared report invariants.
pub(crate) const INCOMPLETE_METADATA: &str =
    "candidate execution metadata is not explicitly complete";
pub(crate) const MISSING_COVERAGE: &str = "complete execution metadata lacks coverage";
pub(crate) const INCONSISTENT_COVERAGE: &str =
    "candidate execution coverage is incomplete or inconsistent";
pub(crate) const INVALID_SKIPS: &str =
    "candidate execution contains invalid or duplicate skipped pairs";
/// Refusal of a publication that carries no samples.
pub const NO_MEASURED_EVIDENCE: &str = "no measured evidence";
/// Refusal of a publication that carries samples of a scoped subset of the inventory.
pub const DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH: &str =
    "diagnostic observations cannot publish benchmark metrics";
