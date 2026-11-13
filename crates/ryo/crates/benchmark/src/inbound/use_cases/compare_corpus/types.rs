//! What one corpus is compared with.
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// One corpus compared rule by rule: both contestants run the same request once.
#[derive(Clone, Copy, Debug)]
pub struct CorpusParity<'a> {
    /// The corpus, its sealed context and the selected rules both contestants run.
    pub request: &'a RunRequest,
    /// The rule set the reference's committed profile configures; the request must select exactly these.
    pub configured: &'a [ryo::linter::RuleId],
    /// The differences accepted for this corpus.
    pub accepted: AcceptedDifferencesFile<'a>,
    /// The reference's name in the published evidence.
    pub reference: ContestantName<'a>,
    /// The rule both contestants report for a source their parser rejects; outside the selection, only
    /// its findings may appear, and only where both contestants report them.
    pub syntax: &'a ryo::linter::RuleId,
}

/// Where the differences accepted for a corpus are written, if anywhere.
#[derive(Clone, Copy, Debug)]
pub enum AcceptedDifferencesFile<'a> {
    /// This file names them.
    Declared(&'a std::path::Path),
    /// None is accepted: every difference fails the comparison.
    Absent,
}

/// The boundaries one corpus comparison calls, borrowed from the use case.
pub(super) struct ParityBoundaries<'a> {
    /// The pinned reference linter.
    pub reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// The candidate linter.
    pub candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// Process launch and observation.
    pub execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    /// Sealed project context.
    pub context: &'a dyn crate::outbound::context::interface::ContextPort,
    /// Accepted differences and source text.
    pub suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    /// Parity metrics and the per-rule document.
    pub evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
}

/// The corpus sources read so far, each read once when a finding in it is first placed.
#[derive(Debug, Default)]
pub(super) struct SourceTexts(
    /// Each source read, by its corpus-relative path.
    pub std::collections::BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
);
