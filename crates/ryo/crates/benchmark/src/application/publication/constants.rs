//! Refusals of a publication.
/// Publishing an empty sample set would claim evidence nobody measured.
pub(super) const NO_MEASURED_EVIDENCE: &str = "no measured evidence";
/// Diagnostic observations carry no benchmark metric.
pub(super) const DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH_BENCH: &str =
    "diagnostic observations cannot publish benchmark metrics";
