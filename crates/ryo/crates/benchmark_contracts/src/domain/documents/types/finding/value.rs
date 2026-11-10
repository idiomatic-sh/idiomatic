//! One diagnostic as a registered case or an observation records it.
use crate::domain::documents::types::text::value::{PathText, RuleName};

/// A byte offset into a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingOffset(pub u64);

/// The first and last byte of a recorded span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordedSpan {
    /// The first byte of the span.
    pub start: FindingOffset,
    /// The byte after the span.
    pub end: FindingOffset,
}

/// A rule finding at an exact file span, as a document recorded it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordedFinding {
    /// The reporting rule.
    pub rule: RuleName,
    /// The file the finding is in.
    pub file: PathText,
    /// Where in the file the finding is.
    pub span: RecordedSpan,
}

/// A rule/file pair skipped after the parser rejected the file's syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedPair {
    /// The skipped rule.
    pub rule: RuleName,
    /// The file the rule was not run on.
    pub path: PathText,
}

/// What the reference is expected to report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpectedFindings {
    /// The reference expectation was derived.
    Derived(Vec<RecordedFinding>),
    /// The reference expectation cannot be derived.
    Underivable,
}
