//! The words of the `patterns` concept.

/// The role of what a pattern element holds.
pub(crate) const BODY: &str = "body";
/// The role of the low end of a character class range.
pub(crate) const MIN: &str = "min";
/// The role of the high end of a character class range.
pub(crate) const MAX: &str = "max";

/// The outcome of a pattern the library parsed.
pub(crate) const PARSED: &str = "parsed";
/// The outcome of a pattern the library refused.
pub(crate) const MALFORMED: &str = "malformed";
/// The outcome of a carrier whose flags the library refused, so that it never read the pattern.
pub(crate) const MALFORMED_FLAGS: &str = "malformed-flags";
