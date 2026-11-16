//! What a corpus is measured with, and what its language tells the measurement.
use crate::domain::measurements::types::LanguageTitle;
use crate::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, SearchRoots};
use ryo_benchmark_contracts::domain::workloads::types::{Project, RuleContextProfile};
use std::path::Path;

/// The pinned reference tool as observations name it.
#[derive(Clone, Copy, Debug)]
pub struct ToolTitle<'a>(pub &'a str);

/// The suffix of a file the matched input profile admits.
#[derive(Clone, Copy, Debug)]
pub struct SourceSuffix<'a>(pub &'a str);

/// The input-profile label an observation carries.
#[derive(Clone, Copy, Debug)]
pub struct ProfileLabel<'a>(pub &'a str);

/// The names and suffixes the language gives its sources, its reference and its profiles.
#[derive(Clone, Copy, Debug)]
pub struct CorpusLanguage<'a> {
    /// The language as benchmark.toml and the bench's measures name it.
    pub id:
        &'a ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
    /// The language as observations name it.
    pub display: LanguageTitle<'a>,
    /// The pinned reference tool as observations name it.
    pub reference_display: ToolTitle<'a>,
    /// The suffixes of the files the matched input profile admits.
    pub suffixes: &'a [SourceSuffix<'a>],
    /// The reference's rule for a source its parser rejects.
    pub invalid_syntax: &'a ryo::linter::RuleId,
    /// The input-profile label of a run limited to accepted sources.
    pub valid_profile: ProfileLabel<'a>,
}

/// What one agreement between two reports compares, named for the refusal that reports a difference.
#[derive(Clone, Copy, Debug)]
pub(super) struct ComparisonName(pub &'static str);

/// The inventory a bench verified its rules in, and the rules it verified.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedInventory<'a> {
    /// The inventory directory the verification wrote.
    pub directory: &'a Path,
    /// The rules it verified.
    pub rules: &'a [ryo::linter::RuleId],
}

/// One certified corpus and everything its measurement is bound to.
#[derive(Clone, Debug)]
pub struct CorpusMeasurement<'a> {
    /// The bench directory whose settings profiles configure the rules.
    pub bench: &'a Path,
    /// The projects measured, in order.
    pub projects: Vec<Project>,
    /// Whether the language grows its rules, and where from.
    pub growth: crate::domain::growth::types::CorpusGrowth<'a>,
    /// The inventory that verified the rules, whose times the bench publishes.
    pub verification: VerifiedInventory<'a>,
    /// The directory retained state and trial directories live under.
    pub cache: &'a Path,
    /// The frozen workload directory the projects' edit patches are read from.
    pub frozen_workload: &'a Path,
    /// The certificate both contestants hold for this corpus.
    pub certificate: &'a Certificate,
    /// Whether the run is accounted or diagnostic.
    pub mode: &'a MeasurementMode,
    /// The rule cohort sent to both contestants.
    pub cohort: &'a CohortSelection,
    /// The profile's rule configuration declaration.
    pub rule_context: &'a RuleContextProfile,
    /// The ordered import roots of every project run.
    pub search_roots: &'a SearchRoots,
    /// How each project's context is sealed.
    pub context_mode: ProjectContextMode,
    /// The names and suffixes of the measured language.
    pub language: CorpusLanguage<'a>,
}

/// The boundaries one corpus measurement calls, borrowed from the use case.
pub(super) struct CorpusBoundaries<'a> {
    /// The pinned reference linter.
    pub reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// The candidate linter.
    pub candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// The reference tool's own selection of a worktree's files.
    pub discovery: &'a dyn crate::outbound::discovery::interface::DiscoveryPort,
    /// Process launch and observation.
    pub execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    /// Artifact validation and fingerprinting.
    pub suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    /// Sealed project context.
    pub context: &'a dyn crate::outbound::context::interface::ContextPort,
    /// Sample publication and comparison evidence.
    pub evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
    /// Corpus worktrees.
    pub workspace: &'a dyn crate::outbound::workspace::interface::WorkspacePort,
    /// The host whose file permissions the profiles may require.
    pub host: &'a dyn crate::outbound::host::interface::HostPort,
    /// The engine the growth programs are linted with, in this process.
    pub engine: &'a dyn crate::outbound::engine::interface::EnginePort,
    /// This process's CPU time and peak, around each growth lint.
    pub meter: &'a dyn crate::outbound::process_meter::interface::ProcessMeterPort,
}

