//! What one corpus is compared with.
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// One corpus compared rule by rule: both contestants run the same request once per corpus.
#[derive(Clone, Copy, Debug)]
pub struct CorpusParity<'a> {
    /// What is compared: one built request, or every project of a language's benchmark.
    pub corpus: Corpus<'a>,
    /// The rule set the reference's committed profile configures; the request must select exactly these.
    pub configured: &'a [ryo::linter::RuleId],
    /// The reference's name in the published evidence; a project's comparison is published as `<project>/<name>`.
    pub reference: ContestantName<'a>,
    /// The rule both contestants report for a source their parser rejects; outside the selection, only
    /// its findings may appear, and only where both contestants report them.
    pub syntax: &'a ryo::linter::RuleId,
}

/// What a parity compares.
#[derive(Clone, Copy, Debug)]
pub enum Corpus<'a> {
    /// One built request, judged by these accepted differences.
    Request {
        /// The corpus, its sealed context and the selected rules both contestants run.
        request: &'a RunRequest,
        /// The differences accepted for this corpus.
        accepted: AcceptedDifferencesFile<'a>,
    },
    /// Every project of a language's benchmark, each checked out at its pin and compared on its own.
    Projects(ProjectsParity<'a>),
}

/// The suffix of a source file a project's comparison admits.
#[derive(Clone, Copy, Debug)]
pub struct SourceExtension<'a>(pub &'a str);

/// The suffix, after a project's name, of the file naming the differences accepted for it.
#[derive(Clone, Copy, Debug)]
pub struct AcceptedSuffix<'a>(pub &'a str);

/// Every project of a language's benchmark: its sources are the ones the reference discovers in its pinned checkout
/// with the language's suffixes, and each request is the one every project run is built with.
#[derive(Clone, Copy, Debug)]
pub struct ProjectsParity<'a> {
    /// The pinned projects, in order.
    pub projects: &'a [ryo_benchmark_contracts::domain::workloads::types::Project],
    /// The suffixes of the sources compared.
    pub suffixes: &'a [SourceExtension<'a>],
    /// The rule cohort sent to both contestants.
    pub cohort: &'a ryo_benchmark_contracts::domain::requests::types::CohortSelection,
    /// The profile's rule configuration declaration.
    pub rule_context: &'a ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile,
    /// The ordered import roots of every run.
    pub search_roots: &'a ryo_benchmark_contracts::domain::requests::types::SearchRoots,
    /// How each project's context is sealed.
    pub context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode,
    /// The directory each project's state directory is made under, by its name.
    pub state: &'a std::path::Path,
    /// The directory each project's accepted-differences file is in; none is accepted when it has none.
    pub accepted: &'a std::path::Path,
    /// The suffix of an accepted-differences file after its project's name.
    pub accepted_suffix: AcceptedSuffix<'a>,
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
    /// The reference tool's own selection of a worktree's files.
    pub discovery: &'a dyn crate::outbound::discovery::interface::DiscoveryPort,
    /// Pinned project checkouts.
    pub workspace: &'a dyn crate::outbound::workspace::interface::WorkspacePort,
}

/// The corpus sources read so far, each read once when a finding in it is first placed.
#[derive(Debug, Default)]
pub(super) struct SourceTexts(
    /// Each source read, by its corpus-relative path.
    pub std::collections::BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
);
