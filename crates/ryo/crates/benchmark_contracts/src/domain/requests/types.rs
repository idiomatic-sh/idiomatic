//! Shared invocation data without scheduling, cache ownership or execution authority.
use crate::domain::context::types::project_context_mode::value::ProjectContextMode;
use crate::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use self::case_shard::value::CaseShard;
use crate::domain::artifacts::types::FileExecutable;
use crate::domain::context::types::{ContextManifest, PreparedRuleContext};
use ryo::linter::{RelativePath, RuleId};
use std::{collections::BTreeMap, path::PathBuf};
/// Both contestants receive exactly this list, never independently walk the tree.
#[derive(Clone, Debug)]
pub struct RunRequest {
    /// Typed physical execution and diagnostic choices.
    pub performance: PerformanceOptions,
    /// Explicit rule cohort for this case; absence preserves the frozen profile selection.
    pub selected_rules: CohortSelection,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: InputDeclaration,
    /// Configuration resolved once by shared request preparation.
    pub rule_context: RuleContextPreparation,
    /// Ordered import roots supplied identically to the candidate and case identity.
    pub project_search_roots: SearchRoots,
    /// Host entry observations supplied only for Given source corpora.
    pub given_project_inventory: ryo::linter::GivenProjectInventory,
    /// Every staged source byte map entry, including contextual files omitted from lint selection.
    pub staged_sources: StagedSources,
    /// Declared file observations, reverified before each contestant is prepared.
    pub file_metadata: FileMetadataDeclaration,
    /// Sealed disk context shared by both contestants, when explicitly selected.
    pub context_manifest: ContextSeal,
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
    pub selected_rules: CohortSelection,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: InputDeclaration,
    /// Explicitly materialized rule configuration for this invocation.
    pub rule_context: RuleContextPreparation,
    /// Ordered import roots; absence means the host did not declare search authority.
    pub project_search_roots: SearchRoots,
    /// Explicit Given corpus entry observations.
    pub given_project_inventory: ryo::linter::GivenProjectInventory,
    /// Whether this invocation may consult the explicitly sealed disk context.
    pub project_context_mode: ProjectContextMode,
    /// Context dependencies sealed before this invocation.
    pub context_manifest: ContextSeal,
    /// Root of the source snapshot read by the candidate.
    pub root: PathBuf,
    /// Directory holding the selected candidate rule directories.
    pub rules: PathBuf,
    /// Logical query-module IDs mapped to explicit profile-relative source paths.
    pub query_modules: BTreeMap<QueryModuleName, PathBuf>,
    /// Exact selected-rule applicability policy forwarded to the isolated candidate worker.
    pub rule_applicability: BTreeMap<RuleLabel, ApplicabilityPolicy>,
    /// Parser configuration identity selected by the frozen profile, if any.
    pub parser_configuration: ParserSelection,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: ProviderSelection,
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
    /// Diagnostic target selection used only with an explicit event destination.
    pub diagnostics: DiagnosticOptions,
}

/// Bounded diagnostic target selection, never read from process environment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticOptions {
    /// Execution events.
    pub execution: DiagnosticSwitch,
    /// Lifecycle memory events.
    pub lifecycle_memory: DiagnosticSwitch,
    /// Resource route events.
    pub resource_routes: DiagnosticSwitch,
    /// Row progress events.
    pub row_progress: DiagnosticSwitch,
    /// Step events.
    pub steps: DiagnosticSwitch,
    /// Join events.
    pub joins: DiagnosticSwitch,
    /// Prefix events.
    pub prefixes: DiagnosticSwitch,
    /// Subplan events.
    pub subplans: DiagnosticSwitch,
    /// Selection detail events.
    pub selection_detail: DiagnosticSwitch,
}

/// Whether one bounded diagnostic target is observed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiagnosticSwitch {
    /// The target is not observed.
    #[default]
    Off,
    /// The target is observed.
    On,
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
    pub shard: ShardSelection,
    /// An explicit subset of IDs assigned to this shard, or the whole shard.
    pub ids: IdSelection<'a>,
}

/// Which partition of the case inventory a verification covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShardSelection {
    /// Every case.
    #[default]
    All,
    /// Exactly the cases of this shard.
    Shard(CaseShard),
}

/// Which durable case IDs of the selected shard a verification covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdSelection<'a> {
    /// The whole shard.
    #[default]
    WholeShard,
    /// Only these case IDs.
    Subset(&'a std::collections::BTreeSet<CaseId>),
}

/// Durable identity of one registered case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaseId(
    /// The case's durable ID text.
    pub String,
);

/// The rule cohort of an invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CohortSelection {
    /// The frozen profile selection applies.
    Frozen,
    /// Exactly these rules are selected.
    Explicit(Vec<RuleId>),
}

/// Named input declarations for the configured consumer queries.
#[derive(Clone, Debug)]
pub enum InputDeclaration {
    /// No declarations are supplied.
    Undeclared,
    /// These declarations are supplied.
    Declared(RuleObservationDeclarations),
}

/// The rule configuration resolved for an invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleContextPreparation {
    /// No rule configuration is materialized.
    Unconfigured,
    /// This configuration is materialized.
    Prepared(PreparedRuleContext),
}

/// Ordered project import roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchRoots {
    /// The host declared no search authority; namespace membership stays unknown.
    Undeclared,
    /// These roots are declared; an empty list is a declared empty scope.
    Declared(Vec<RelativePath>),
}

/// The staged source map entries, including contextual files omitted from lint selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StagedSources {
    /// Only the lint selection is staged.
    Unlisted,
    /// Every staged entry.
    Listed(Vec<RelativePath>),
}

/// Declared POSIX execute bits of the staged files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileMetadataDeclaration {
    /// No execute bits are declared; identity is byte-only.
    Undeclared,
    /// The execute bit of each declared file.
    Declared(BTreeMap<RelativePath, FileExecutable>),
}

/// The sealed disk context of an invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextSeal {
    /// No disk context is sealed.
    Unsealed,
    /// This context is sealed.
    Sealed(ContextManifest),
}

/// Identity of a parser configuration shared by reference and candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserConfigurationName(
    /// The configuration's identity text.
    pub String,
);

/// The parser configuration selected by the frozen profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParserSelection {
    /// The tool's parser defaults apply.
    Unspecified,
    /// This configuration applies.
    Named(ParserConfigurationName),
}

/// Identity of an explicit source provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLanguageName(
    /// The provider's language identity text.
    pub String,
);

/// The source provider selected by the request edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderSelection {
    /// No provider is selected explicitly.
    Unspecified,
    /// This provider is selected.
    Selected(SourceLanguageName),
}

/// Logical identity of a query module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueryModuleName(
    /// The module's logical ID text.
    pub String,
);

/// Text identity of a rule as the benchmark's artifacts spell it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleLabel(
    /// The rule's identifier text.
    pub String,
);

/// Pinned phase policy of an authored candidate rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicabilityPolicy(
    /// The policy's spelling.
    pub String,
);

pub mod case_shard;
