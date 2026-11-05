//! Shared invocation data without scheduling, cache ownership or execution authority.
use crate::domain::context::types::project_context_mode::value::ProjectContextMode;
use crate::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use crate::domain::context::types::{ContextManifest, PreparedRuleContext};
use ryo::linter::{RelativePath, RuleId};
use std::{collections::BTreeMap, path::PathBuf};
/// Both contestants receive exactly this list, never independently walk the tree.
#[derive(Clone, Debug)]
pub struct RunRequest {
    /// Typed physical execution and diagnostic choices.
    pub performance: PerformanceOptions,
    /// Explicit rule cohort for this case; absence preserves the frozen profile selection.
    pub selected_rules: Option<Vec<RuleId>>,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: Option<RuleObservationDeclarations>,
    /// Configuration resolved once by shared request preparation.
    pub rule_context: Option<PreparedRuleContext>,
    /// Ordered import roots supplied identically to the candidate and case identity.
    pub project_search_roots: Option<Vec<RelativePath>>,
    /// Host entry observations supplied only for Given source corpora.
    pub given_project_inventory: ryo::linter::GivenProjectInventory,
    /// Every staged source byte map entry, including contextual files omitted from lint selection.
    pub staged_sources: Option<Vec<RelativePath>>,
    /// Declared file observations, reverified before each contestant is prepared.
    pub file_metadata: Option<BTreeMap<RelativePath, bool>>,
    /// Sealed disk context shared by both contestants, when explicitly selected.
    pub context_manifest: Option<ContextManifest>,
    /// Root of the exact source snapshot supplied to both contestants.
    pub root: PathBuf,
    /// Shared explicit list of relative source paths.
    pub files: Vec<RelativePath>,
    /// Directory beneath which contestants keep their separate caches.
    pub state: PathBuf,
}

/// Inputs supplied to the SDK worker inside the measured child.
#[derive(Clone, Debug)]
pub struct WorkerRequest {
    /// Typed execution and diagnostic choices, included in invocation identity.
    pub performance: PerformanceOptions,
    /// Explicit rule cohort for this case; absence preserves the frozen profile selection.
    pub selected_rules: Option<Vec<RuleId>>,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: Option<RuleObservationDeclarations>,
    /// Explicitly materialized rule configuration for this invocation.
    pub rule_context: Option<PreparedRuleContext>,
    /// Ordered import roots; absence means the host did not declare search authority.
    pub project_search_roots: Option<Vec<RelativePath>>,
    /// Explicit Given corpus entry observations.
    pub given_project_inventory: ryo::linter::GivenProjectInventory,
    /// Whether this invocation may consult the explicitly sealed disk context.
    pub project_context_mode: ProjectContextMode,
    /// Context dependencies sealed before this invocation.
    pub context_manifest: Option<ContextManifest>,
    /// Root of the source snapshot read by the candidate.
    pub root: PathBuf,
    /// Directory holding the selected candidate rule directories.
    pub rules: PathBuf,
    /// Logical query-module IDs mapped to explicit profile-relative source paths.
    pub query_modules: BTreeMap<String, PathBuf>,
    /// Exact selected-rule applicability policy forwarded to the isolated candidate worker.
    pub rule_applicability: BTreeMap<String, String>,
    /// Parser configuration identity selected by the frozen profile, if any.
    pub parser_configuration: Option<String>,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: Option<String>,
    /// Explicit relative paths the worker must check.
    pub files: Vec<RelativePath>,
    /// Optional persistent SDK cache directory; absence disables disk caching.
    pub state: WorkerState,
}

/// Whether a worker observation starts fresh or reuses explicitly retained state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerState {
    /// Do not retain disk state between observations.
    Fresh,
    /// Reuse only this observation's explicitly selected state directory.
    Retained(PathBuf),
}

/// Explicit performance controls; diagnostics are independent from timed execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerformanceOptions {
    /// Physical strategy forwarded through the SDK composition edge.
    pub execution: ryo::linter::ExecutionOptions,
    /// Diagnostic target selection used only with an explicit event destination.
    pub diagnostics: DiagnosticOptions,
}

/// Bounded diagnostic target selection, never read from process environment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticOptions {
    pub execution: bool,
    pub lifecycle_memory: bool,
    pub resource_routes: bool,
    pub row_progress: bool,
    pub steps: bool,
    pub joins: bool,
    pub prefixes: bool,
    pub subplans: bool,
    pub selection_detail: bool,
}

/// Whether the full inventory has been assessed or this is an interim authored subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkCoverage {
    /// Every default is implemented or linked to an assessed capability deferral.
    Accounted,
    /// Unassessed defaults remain explicit and the certificate covers only authored rules.
    Interim,
}

/// Independent cases retain the context in which their reference contract was registered.
#[derive(Clone, Debug)]
pub enum CertificationCases {
    /// One self-contained profile, used for focused adapter/diagnostic certificates.
    Profile,
    /// Existing shared inventory runner owns all registered native case executions.
    Registered {
        registration: PathBuf,
        output: PathBuf,
    },
}

/// Optional partition and durable case IDs for a verification invocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaseSelection<'a> {
    /// Globally audited deterministic shard, or all cases.
    pub shard: Option<case_shard::value::CaseShard>,
    /// An explicit subset of IDs assigned to this shard, or the whole shard.
    pub ids: Option<&'a std::collections::BTreeSet<String>>,
}

pub mod case_shard;
