//! Canonical findings use relative paths and UTF-8 byte offsets.
use crate::domain::context::types::ProjectContextReceipt;
use ryo::linter::RelativePath;
use ryo::linter::RuleId;

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
    pub project_context: ProjectContext,
    /// Finding multiset, preserving duplicate occurrences.
    pub diagnostics: Vec<BenchmarkFinding>,
    /// Rules explicitly skipped for syntax applicability, kept separate from empty findings.
    pub skipped: Vec<ryo::linter::Skipped>,
    /// Candidate execution accounting; external references may not expose this distinction.
    pub coverage: CoverageAccounting,
    /// Exact relative source paths submitted to the tool.
    pub files: Vec<RelativePath>,
}

/// The project lookups of a report, when explicitly enabled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectContext {
    /// No project context was requested, so no lookup was observed.
    NotRequested,
    /// The lookups actually used and their sealed snapshot identity.
    Observed(ProjectContextReceipt),
}

/// Whether a report accounts for the rule/file pairs it selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageAccounting {
    /// The producer exposes no per-pair accounting.
    Unaccounted,
    /// Every selected pair is accounted for.
    Accounted(RuleCoverage),
}

/// Format-level reason a producer cannot expose execution outcomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnavailableReason(
    /// The limitation, without inferred execution semantics.
    pub String,
);

/// What a report's producer can establish about per-rule execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ExecutionMetadata {
    /// A legacy or undeclared format provides no capability evidence.
    #[default]
    Unknown,
    /// The declared format cannot expose per-rule execution outcomes.
    Unavailable {
        /// Format-level limitation, without inferred execution semantics.
        reason: UnavailableReason,
    },
    /// Coverage and every skipped pair are explicitly reported and must validate.
    Complete,
}

/// Agreement for one independently comparable report dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    /// Both sides expose equivalent evidence.
    Exact,
    /// Comparable evidence differs, or a required candidate invariant failed.
    Different,
    /// At least one side cannot expose the evidence needed for comparison.
    Incomparable,
}

/// Finding parity and execution parity are separate claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportComparison {
    /// Exact diagnostic multisets and submitted files.
    pub diagnostics: Comparison,
    /// Explicit coverage and skipped rule/file pairs, never inferred from findings.
    pub execution_metadata: Comparison,
}

/// The evidence a caller requires before accepting paired observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub selected: PairCount,
    /// Selected pairs whose query ran.
    pub evaluated: PairCount,
    /// Pairs skipped after explicit syntax rejection.
    pub skipped: PairCount,
    /// Pairs unavailable because the engine returned unknown or unread state.
    pub unknown_or_unread: PairCount,
}

/// A number of rule/file pairs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PairCount(
    /// Count of rule/file pairs.
    pub usize,
);

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

/// External artifact observations, not an audit or a parity certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoredInventory {
    /// Declared default IDs with a query and validated case envelope.
    pub implemented: Vec<RuleId>,
    /// Declared default IDs whose query is absent.
    pub missing: Vec<RuleId>,
}
