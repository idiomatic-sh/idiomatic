//! Stable operation vocabulary and artifact identities.
pub(super) const CHANGED_EVIDENCE: &str = "certified profile, rules, cases or executable changed";
pub(super) const CHANGED_COMMAND: &str =
    "contestant command profile differs from case certification";
pub(super) const CHANGED_EXECUTABLE: &str = "contestant differs from case-certified executable";
pub(super) const CHANGED_DURING_RUN: &str = "certified artifacts changed during paired observation";
pub(super) const CHANGED_SOURCE: &str = "corpus source changed during paired observation";
pub(super) const CHANGED_INSTRUMENTATION: &str =
    "execution profile differs from case certification";
