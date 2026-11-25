//! Stable operation vocabulary and artifact identities.
/// Authored rule directory sealed by certification.
pub(crate) const RULE_DIRECTORY: &str = "rules";
/// Refuse changed content fingerprints of certified artifacts.
pub(crate) const CHANGED_EVIDENCE: &str = "certification inputs changed during cases";
/// Refuse an identity probe whose preparation changed a sealed certification input.
pub(crate) const PROBE_CHANGED_INPUTS: &str =
    "the identity probe changed a sealed certification input";

// Independent executable fixture inputs and expected refusal vocabulary.

/// A selected registered coverage never stands for the accounted inventory.
pub(crate) const SELECTED_COVERAGE_REQUIRES_INTERIM: &str =
    "selected registered coverage certifies only an interim run";
