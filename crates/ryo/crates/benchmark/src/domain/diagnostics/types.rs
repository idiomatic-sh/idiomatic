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
    /// Rules explicitly skipped for syntax applicability, kept separate from empty findings.
    pub skipped: Vec<ryo::linter::Skipped>,
    /// Candidate execution accounting; external references may not expose this distinction.
    pub coverage: Option<RuleCoverage>,
    /// Exact relative source paths submitted to the tool.
    pub files: Vec<RelativePath>,
}

/// Accounting for each selected rule/file pair in one candidate execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleCoverage {
    /// Selected rule/file pairs before applicability checks.
    pub selected: usize,
    /// Selected pairs whose query ran.
    pub evaluated: usize,
    /// Pairs skipped after explicit syntax rejection.
    pub skipped: usize,
    /// Pairs unavailable because the engine returned unknown or unread state.
    pub unknown_or_unread: usize,
}

/// Sorted finding multisets retained when comparison does not agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Sorted findings returned by the reference.
    pub reference: Vec<BenchmarkFinding>,
    /// Sorted findings returned by the candidate.
    pub candidate: Vec<BenchmarkFinding>,
    /// Reference-side applicability skips, normally empty for external tools.
    pub reference_skipped: Vec<ryo::linter::Skipped>,
    /// Candidate-side applicability skips, never treated as an empty clean result.
    pub candidate_skipped: Vec<ryo::linter::Skipped>,
}

/// Position in the source byte stream, distinct from Unicode columns or query offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceByteOffset(
    /// Zero-based UTF-8 byte position within the source.
    pub usize,
);
