//! Shared diagnostic refusal vocabulary.
/// Repeated corpus paths would make coverage and multiplicity ambiguous.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate input path";
/// Cross-linter identity used for parser diagnostics in complete benchmark reports.
pub(crate) const FRONTEND_SYNTAX: &str = "invalid-syntax";
/// Refuse a contestant report that does not cover exactly the requested files.
pub(crate) const INPUT_COVERAGE: &str = "tool input coverage differs from requested corpus";
/// Refuse a byte offset past the end of its source or inside a UTF-8 character.
pub(crate) const OFFSET_OUTSIDE_SOURCE: &str =
    "byte offset is outside the source or inside a character";
