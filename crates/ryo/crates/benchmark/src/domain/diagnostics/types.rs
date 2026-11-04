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
    /// Whether the producer actually reports rule/file execution outcomes; absent evidence is never known zero.
    pub execution_metadata: ExecutionMetadata,
    /// Actual project lookups and their sealed snapshot identity, when explicitly enabled.
    pub project_context: Option<ProjectContextReceipt>,
    /// Finding multiset, preserving duplicate occurrences.
    pub diagnostics: Vec<BenchmarkFinding>,
    /// Rules explicitly skipped for syntax applicability, kept separate from empty findings.
    pub skipped: Vec<ryo::linter::Skipped>,
    /// Candidate execution accounting; external references may not expose this distinction.
    pub coverage: Option<RuleCoverage>,
    /// Exact relative source paths submitted to the tool.
    pub files: Vec<RelativePath>,
}

/// What a report's producer can establish about per-rule execution.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExecutionMetadata {
    /// A legacy or undeclared format provides no capability evidence.
    #[default]
    Unknown,
    /// The declared format cannot expose per-rule execution outcomes.
    Unavailable {
        /// Format-level limitation, without inferred execution semantics.
        reason: String,
    },
    /// Coverage and every skipped pair are explicitly reported and must validate.
    Complete,
}

/// Agreement for one independently comparable report dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Comparison {
    /// Both sides expose equivalent evidence.
    Exact,
    /// Comparable evidence differs, or a required candidate invariant failed.
    Different,
    /// At least one side cannot expose the evidence needed for comparison.
    Incomparable,
}

/// Finding parity and execution parity are separate claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ReportComparison {
    /// Exact diagnostic multisets and submitted files.
    pub diagnostics: Comparison,
    /// Explicit coverage and skipped rule/file pairs, never inferred from findings.
    pub execution_metadata: Comparison,
}

/// The evidence a caller requires before accepting paired observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonContract {
    /// All diagnostics and submitted files; execution evidence may remain explicitly incomparable.
    Diagnostics,
    /// Diagnostics and complete execution metadata must both agree.
    CompleteReport,
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

/// The context actually used by one candidate invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectContextReceipt {
    /// Explicit candidate isolation mode.
    pub mode: crate::domain::suite::types::ProjectContextMode,
    /// Run root's stable position within the materialized snapshot.
    pub logical_root: std::path::PathBuf,
    /// Actual physical run directory supplied to both tools.
    pub actual_root: std::path::PathBuf,
    /// Hash of declared files, physically validated observed absences and logical root.
    pub manifest_hash: crate::domain::suite::types::ArtifactDigest,
    /// Every observed dependency, including those retained by warm reuse.
    pub dependencies:
        std::collections::BTreeMap<std::path::PathBuf, Option<ryo::linter::DependencyFingerprint>>,
}

/// Sorted finding multisets retained when comparison does not agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Sorted findings returned by the reference.
    pub reference: Vec<BenchmarkFinding>,
    /// Sorted findings returned by the candidate.
    pub candidate: Vec<BenchmarkFinding>,
    /// Raw reference-side skip rows; an empty vector cannot establish producer capability.
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
