//! Stable operation vocabulary and artifact identities.
/// Refuse changed content fingerprints of certified artifacts.
pub(super) const CHANGED_EVIDENCE: &str = "certified profile, rules, cases or executable changed";
/// Refuse prepared command settings that differ from case verification.
pub(super) const CHANGED_COMMAND: &str =
    "contestant command profile differs from case certification";
/// Refuse a different contestant executable.
pub(super) const CHANGED_EXECUTABLE: &str = "contestant differs from case-certified executable";
/// Refuse artifacts mutated during paired execution.
pub(super) const CHANGED_DURING_RUN: &str = "certified artifacts changed during paired observation";
/// Refuse source bytes mutated during paired execution.
pub(super) const CHANGED_SOURCE: &str = "corpus source changed during paired observation";
/// Refuse a different resource collection profile.
pub(super) const CHANGED_INSTRUMENTATION: &str =
    "execution profile differs from case certification";
