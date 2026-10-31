//! Canonical findings use relative paths and UTF-8 byte offsets.
use ryo::linter::{RelativePath, RuleId};

/// One normalized finding. Duplicate values remain significant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkFinding {
    /// Rule identifier within the pinned default profile.
    pub rule: RuleId,
    /// Relative path of the source containing the finding.
    pub file: RelativePath,
    /// Inclusive start offset in UTF-8 bytes.
    pub start: SourceByteOffset,
    /// Exclusive end offset in UTF-8 bytes.
    pub end: SourceByteOffset,
}

/// Decoded findings and the exact files submitted to the tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    /// Finding multiset, preserving duplicate occurrences.
    pub diagnostics: Vec<BenchmarkFinding>,
    /// Exact relative source paths submitted to the tool.
    pub files: Vec<RelativePath>,
}

/// Sorted finding multisets retained when comparison does not agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Sorted findings returned by the reference.
    pub reference: Vec<BenchmarkFinding>,
    /// Sorted findings returned by the candidate.
    pub candidate: Vec<BenchmarkFinding>,
}

/// Position in the source byte stream, distinct from Unicode columns or query offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceByteOffset(pub usize);
