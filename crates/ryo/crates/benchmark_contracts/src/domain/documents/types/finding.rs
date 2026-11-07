//! One diagnostic as a registered case or an observation records it.
use super::text::{PathText, RuleName};

/// A byte offset into a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingOffset(pub u64);

/// A rule finding at an exact file span.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Finding {
    /// The reporting rule.
    pub rule: RuleName,
    /// The file the finding is in.
    pub file: PathText,
    /// The first byte of the span.
    pub start: FindingOffset,
    /// The byte after the span.
    pub end: FindingOffset,
}

/// A rule/file pair skipped after the parser rejected the file's syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedPair {
    /// The skipped rule.
    pub rule: RuleName,
    /// The file the rule was not run on.
    pub path: PathText,
}
