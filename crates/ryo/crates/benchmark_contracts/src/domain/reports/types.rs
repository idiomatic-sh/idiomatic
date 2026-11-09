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

/// A one-based source line, lines split on `\r\n`, `\r` or `\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceLine(
    /// The one-based line number.
    pub usize,
);

/// A one-based column counted in Unicode characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceColumn(
    /// The one-based character column.
    pub usize,
);

/// A place in a source as a person reads it: a line and a character column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindingPosition {
    /// The line.
    pub line: SourceLine,
    /// The column within the line.
    pub column: SourceColumn,
}

/// One finding placed by line and column rather than bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatedFinding {
    /// Relative path of the source containing the finding.
    pub file: RelativePath,
    /// Where the finding starts.
    pub start: FindingPosition,
    /// Where the finding ends.
    pub end: FindingPosition,
}

/// A candidate and a reference finding paired at one file and start whose ends differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndPair<Finding> {
    /// The candidate's finding.
    pub candidate: Finding,
    /// The reference's finding.
    pub reference: Finding,
}

/// How many findings paired on file and start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindingCount(
    /// The number of pairs.
    pub usize,
);

/// One selected rule's findings compared as a multiset on file and start; findings at one site pair in
/// report order. The candidate reported `matched` plus its extra findings, the reference `matched` plus
/// its missing ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleComparison<Finding> {
    /// The selected rule.
    pub rule: RuleId,
    /// How many findings paired.
    pub matched: FindingCount,
    /// The reference findings with no candidate pair, in file and start order.
    pub missing: Vec<Finding>,
    /// The candidate findings with no reference pair, in file and start order.
    pub extra: Vec<Finding>,
    /// The pairs whose ends differ, in file and start order.
    pub end_differences: Vec<EndPair<Finding>>,
}

/// The name a contestant's counts carry in parity evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContestantName<'a>(
    /// The name.
    pub &'a str,
);

/// One difference of a corpus comparison that no acceptance covers, as a person reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParityProblem(
    /// The problem, naming the rule and the finding.
    pub String,
);

/// Why a rule's differences on a corpus are accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceReason(
    /// The explanation, with its evidence.
    pub String,
);

/// Two contestants compared rule by rule over one corpus, with the differences no acceptance covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParityReport<'a> {
    /// The pinned reference's name.
    pub reference: ContestantName<'a>,
    /// The candidate's name.
    pub candidate: ContestantName<'a>,
    /// Each selected rule, in selection order.
    pub rules: Vec<RuleComparison<LocatedFinding>>,
    /// Each difference the accepted differences do not cover.
    pub problems: Vec<ParityProblem>,
}

/// The side of an unmatched finding in a corpus comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DifferenceSide {
    /// Only the candidate reports it.
    Extra,
    /// Only the reference reports it.
    Missing,
}

/// One unmatched finding of a rule, pinned by its side, file and start.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocatedDifference {
    /// Which contestant alone reports it.
    pub side: DifferenceSide,
    /// Relative path of the source containing it.
    pub file: RelativePath,
    /// Where it starts.
    pub start: FindingPosition,
}

/// The differences of one rule a corpus comparison accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedRule {
    /// The rule.
    pub rule: RuleId,
    /// Why these differences are accepted.
    pub reason: AcceptanceReason,
    /// The accepted unmatched findings.
    pub findings: Vec<LocatedDifference>,
}

/// The differences a corpus comparison accepts, written for one corpus digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedDifferences {
    /// The digest of the corpus the differences were observed on.
    pub corpus: crate::domain::artifacts::types::digest::value::ArtifactDigest,
    /// The accepted differences of each rule.
    pub rules: Vec<AcceptedRule>,
}
