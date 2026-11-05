//! Stable operation vocabulary and artifact identities.
/// Refuse a contestant report that does not cover exactly the requested files.
pub(super) const INPUT_COVERAGE: &str = "tool input coverage differs from requested corpus";
/// Reference contestant label in differential failures.
pub(super) const REFERENCE: &str = "reference";
/// Candidate contestant label in differential failures.
pub(super) const CANDIDATE: &str = "candidate";
/// Separate complete fixture failure records.
pub(super) const LINE_SEPARATOR: &str = "\n";

/// Authored rule directory within the frozen profile.

/// Refuse certification without executable cases.
pub(super) const NO_CASES: &str = "no executable rule cases";
