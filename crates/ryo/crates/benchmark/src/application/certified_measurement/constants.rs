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
/// What a project's per-rule judgement reports first.
pub(super) const UNACCEPTED: &str = "unaccepted differences";
/// Where a bench keeps the differences accepted for its projects, relative to the bench.
pub(super) const ACCEPTED_DIRECTORY: &str = "profiles/evidence";
/// The suffix of a project's accepted-differences file, after the project's name.
pub(super) const ACCEPTED_SUFFIX: &str = "-accepted-differences.json";
/// Between two differences of one refusal.
pub(super) const DIFFERENCE_SEPARATOR: &str = "; ";
/// The directory below a run's state that the reference keeps its own state in.
pub(super) const REFERENCE_STATE: &str = "reference";
/// The directory below a run's state that the candidate keeps its own state in: the store measured.
pub(super) const CANDIDATE_STATE: &str = "candidate";
