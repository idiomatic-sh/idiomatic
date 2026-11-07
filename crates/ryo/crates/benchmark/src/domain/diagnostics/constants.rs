//! Shared diagnostic refusal vocabulary.
/// Repeated corpus paths would make coverage and multiplicity ambiguous.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate input path";
/// Cross-linter identity used for parser diagnostics in complete benchmark reports.
pub(crate) const FRONTEND_SYNTAX: &str = "invalid-syntax";
/// Refuse a contestant report that does not cover exactly the requested files.
pub(crate) const INPUT_COVERAGE: &str = "tool input coverage differs from requested corpus";
