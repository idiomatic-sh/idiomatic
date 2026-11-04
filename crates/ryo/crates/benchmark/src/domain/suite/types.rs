//! Immutable inputs and independently pinned shipped defaults.
use crate::domain::diagnostics::types::SourceByteOffset;
use ryo::linter::{RelativePath, RuleId, SourceText};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Shared project inputs for one language benchmark.
#[derive(Clone, Debug)]
pub struct Suite {
    /// The language identifier shared by the project inputs.
    pub language: SuiteLanguage,
    /// Pinned repositories used for the common corpus.
    pub projects: Vec<Project>,
}

/// One repository pinned to an immutable commit and a deterministic edit.
#[derive(Clone, Debug)]
pub struct Project {
    /// Stable project identifier used to name its isolated checkout.
    pub name: ProjectName,
    /// Git remote from which the pinned commit is fetched.
    pub repository: RepositoryUrl,
    /// Full immutable Git commit identifier.
    pub revision: ProjectRevision,
    /// Edit patch path relative to the language benchmark directory.
    pub patch: PathBuf,
}

/// Explicit consumer declarations, keyed by the actual configured rule identity.
pub type RuleObservationDeclarations = BTreeMap<RuleId, Vec<ryo::linter::ObservationBinding>>;

/// Positive configuration knowledge, distinct from whether an executable producer completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleConfiguration {
    /// Literal prefixes the tool/query treats as external rule identifiers.
    pub external_prefixes: Vec<String>,
    /// Exact requested relative files and the rules disabled there; these are not glob patterns.
    pub disabled_rules: BTreeMap<RelativePath, Vec<RuleId>>,
}
/// Explicit names of existing hosted request-scope configuration tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleContextTables {
    /// Selected configured rule IDs, one text column `code`.
    pub selected: String,
    /// Disabled file/rule pairs, columns `file` and `code`.
    pub disabled: String,
    /// Declared external prefixes, one text column `prefix`.
    pub external: String,
}
/// A profile opts into configuration inputs with required table names and explicit defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleContextDeclaration {
    /// Required semantic table names; no implicit fallback names exist.
    pub tables: RuleContextTables,
    /// Complete declared configuration when a case supplies no override.
    pub defaults: RuleConfiguration,
}
/// The resolved data transported to one candidate invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedRuleContext {
    /// Actual declared table names.
    pub tables: RuleContextTables,
    /// Complete configuration chosen from the same profile/case data as the reference.
    pub configuration: RuleConfiguration,
}

/// The independently pinned reference executable and its shipped defaults.
#[derive(Clone, Debug)]
pub struct Tool {
    /// Explicit executable support definitions; never default selection or case-owner credit.
    pub supporting_rules: Vec<RuleId>,
    /// Default authored named inputs; case overrides are explicit and source-bound.
    pub rule_observations: RuleObservationDeclarations,
    /// Optional explicitly declared request-scope rule configuration inputs.
    pub rule_context: Option<RuleContextDeclaration>,
    /// File observation this correctness profile requires for every declared source.
    pub file_observation: Option<FileObservation>,
    /// Whether the candidate receives an isolated input or an explicitly sealed disk context.
    pub project_context_mode: ProjectContextMode,
    /// Reference executable identity reported by its version command.
    pub tool: ToolName,
    /// Exact reference version required by this suite.
    pub version: ToolVersion,
    /// Independently pinned native executable digests for supported platforms.
    pub sha256: Vec<ToolDigest>,
    /// Exact shipped-default rule identifiers for the pinned reference profile.
    pub defaults: Vec<RuleId>,
    /// Deferred default rules mapped to a public capability identifier.
    pub deferred: BTreeMap<RuleId, CapabilityName>,
    /// Assessed tractable work still pending implementation, never counted as a deferral.
    pub pending: BTreeMap<RuleId, CapabilityName>,
    /// Explicit logical query-module IDs mapped to safe profile-relative source paths.
    pub query_modules: BTreeMap<String, PathBuf>,
    /// Pinned phase policy for authored candidate rules; absent entries retain legacy Always.
    pub rule_applicability: BTreeMap<String, String>,
    /// Optional parser configuration that must be shared by reference and candidate.
    pub parser_configuration: Option<String>,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: Option<String>,
}

/// Generic input observation required by a profile's comparison contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileObservation {
    /// Actual POSIX execute bits must be declared, staged and observed.
    PosixExecutable,
}

/// The correctness cases authored beside one candidate query.
#[derive(Clone, Debug)]
pub struct Cases {
    /// Named positive and negative cases for the enclosing rule.
    pub cases: Vec<Case>,
}

/// Diagnostic projection asserted by an independent case.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaseComparison {
    /// Assert findings belonging to the enclosing rule.
    #[default]
    Rule,
    /// Assert every diagnostic, including frontend errors. This describes diagnostic scope, not execution-metadata parity.
    Full,
}

/// Explicit diagnostic identity or the enclosing rule's identity.
#[derive(Clone, Debug)]
pub enum ExpectedRule {
    /// Existing rule-scoped fixture shorthand.
    Parent,
    /// Independently specified diagnostic identity.
    Explicit(RuleId),
}

/// An isolated set of source files and its exact expected findings.
#[derive(Clone, Debug)]
pub struct Case {
    /// Explicit execution and diagnostic policy for this case.
    pub performance: PerformanceOptions,
    /// Explicit rule cohort for this case; absence preserves the frozen profile selection.
    pub selected_rules: Option<Vec<RuleId>>,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: Option<RuleObservationDeclarations>,
    /// Explicit complete override of the profile rule configuration.
    pub rule_context: Option<RuleConfiguration>,
    /// Explicit primary inventory, including selected paths sealed absent by the context.
    /// None preserves the legacy inventory of exactly the source-text map's keys.
    pub requested_files: Option<Vec<RelativePath>>,
    /// Explicit per-file POSIX executable bits; None preserves legacy byte-only identity.
    pub file_metadata: Option<BTreeMap<RelativePath, bool>>,
    /// Explicit logical root and auxiliary context, excluded from the lint file inventory.
    pub context: Option<CaseContext>,
    /// Whether all diagnostics or only the enclosing rule are asserted.
    pub comparison: CaseComparison,
    /// Human-readable identity of this correctness case.
    pub name: BenchmarkCaseName,
    /// Relative source paths and their complete UTF-8 contents.
    pub files: BTreeMap<RelativePath, SourceText>,
    /// Expected finding multiset; an empty list is a negative case.
    pub expected: Vec<Expected>,
    /// Optional pinned reference behavior that intentionally differs from the candidate contract.
    pub reference_discrepancy: Option<ReferenceDiscrepancy>,
}

/// Evidence for a documented reference defect, never a wildcard diagnostic exception.
#[derive(Clone, Debug)]
pub struct ReferenceDiscrepancy {
    /// Exact reference diagnostics; `Case.expected` remains the intended candidate contract.
    pub expected: Vec<Expected>,
    /// Tool/version/profile/settings/source and both canonical output identities.
    pub identity: ReferenceDiscrepancyIdentity,
    /// Independent language/specification observations, checked by content digest.
    pub evidence: Vec<DiscrepancyEvidence>,
    /// Explanation of the independently intended behavior and reference disagreement.
    pub reason: String,
}

/// Immutable identities required before admitting a reference discrepancy.
#[derive(Clone, Debug)]
pub struct ReferenceDiscrepancyIdentity {
    /// Separate declared-metadata identity; absent only for legacy unspecified observations.
    pub file_metadata_sha256: Option<ArtifactDigest>,
    /// Reference tool name.
    pub tool: ToolName,
    /// Exact reference version.
    pub version: ToolVersion,
    /// Executable used to observe the defect; must be an approved profile pin.
    pub observed_executable_sha256: ArtifactDigest,
    /// Exact profile bytes, including all approved platform executable pins.
    pub profile_sha256: ArtifactDigest,
    /// Normalized effective reference invocation and parser/context settings.
    pub settings_sha256: ArtifactDigest,
    /// Exact authored relative filenames and source bytes.
    pub source_sha256: ArtifactDigest,
    /// Exact normalized expected reference report.
    pub reference_output_sha256: ArtifactDigest,
    /// Exact normalized intended candidate report; changing expectations invalidates evidence.
    pub intended_output_sha256: ArtifactDigest,
}

/// Independent authority accepted for a reference discrepancy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscrepancyEvidenceKind {
    /// A versioned language specification artifact.
    LanguageSpec,
    /// An independent language implementation observation.
    IndependentLanguageRun,
}

/// The authority asserted by a documented two-sided difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscrepancyBasis {
    /// Independent language meaning contradicts the reference output.
    LanguageDefect,
    /// A documented public tool policy differs from the pinned reference tool.
    ToolContract,
}

/// One exact artifact within the profile boundary, with evidence fields specific to its authority.
#[derive(Clone, Debug)]
pub enum DiscrepancyEvidence {
    /// Independent language meaning.
    Language {
        /// Safe profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact content digest.
        sha256: ArtifactDigest,
        /// Language specification or execution.
        kind: DiscrepancyEvidenceKind,
        /// Observed language version.
        language_version: String,
        /// Parser target supported by this evidence.
        target_configuration: String,
        /// Connection between observation and target.
        version_rationale: String,
    },
    /// A pinned previously published public tool contract.
    PublicToolContract {
        /// Safe profile-relative contract snapshot.
        path: PathBuf,
        /// Exact snapshot digest.
        sha256: ArtifactDigest,
        /// Published source revision.
        source_revision: String,
        /// Public source path.
        source_path: String,
        /// Case-specific policy explanation.
        rationale: String,
    },
    /// Actual prior worker execution of the same policy boundary.
    PriorWorkerRun {
        /// Safe profile-relative complete observation.
        path: PathBuf,
        /// Exact observation digest.
        sha256: ArtifactDigest,
        /// Sealed worker identity.
        worker_sha256: ArtifactDigest,
        /// Exact control input, query and profile identities; these need not equal the rule case.
        control_source_sha256: ArtifactDigest,
        /// Exact control query identity.
        control_query_sha256: ArtifactDigest,
        /// Exact control profile identity.
        control_profile_sha256: ArtifactDigest,
        /// Named completed control that must exhibit the prior policy.
        witness_case: String,
        /// Why the separate control proves this selection boundary.
        rationale: String,
    },
}
impl DiscrepancyEvidence {
    /// The profile-contained artifact identity common to every authority.
    #[must_use]
    pub fn artifact(&self) -> (&PathBuf, &ArtifactDigest) {
        match self {
            Self::Language { path, sha256, .. }
            | Self::PublicToolContract { path, sha256, .. }
            | Self::PriorWorkerRun { path, sha256, .. } => (path, sha256),
        }
    }
}

/// Materialized reference expectation with independently validated evidence.
#[derive(Clone, Debug)]
pub struct PreparedReferenceDiscrepancy {
    /// Language defect or deliberately different public tool policy.
    pub basis: DiscrepancyBasis,
    /// Exact reference report; distinct from the candidate's intended expected report.
    pub expected: super::process::types::ReferenceExpectation,
    /// Immutable execution and output identities.
    pub identity: ReferenceDiscrepancyIdentity,
    /// Independent evidence retained for reporting and stale-artifact refusal.
    pub evidence: Vec<DiscrepancyEvidence>,
    /// Human-readable explanation retained beside raw parity mismatches.
    pub reason: String,
}

/// An independently authored case materialized by the suite boundary.
#[derive(Clone, Debug)]
pub struct PreparedCase {
    /// Diagnostic projection selected by the authored case.
    pub comparison: CaseComparison,
    /// Selected default rule whose findings this case asserts.
    pub rule: RuleId,
    /// Unique case name within that rule.
    pub name: BenchmarkCaseName,
    /// Exact immutable source inputs supplied to both contestants.
    pub request: RunRequest,
    /// Independently authored findings and input coverage.
    pub expected: crate::domain::diagnostics::types::RunReport,
    /// Optional separate reference expectation; never changes the intended candidate output.
    pub reference_discrepancy: Option<PreparedReferenceDiscrepancy>,
}

/// One expected finding in a case, located by UTF-8 byte offsets.
#[derive(Clone, Debug)]
pub struct Expected {
    /// Diagnostic identity; full-report cases must specify it explicitly.
    pub identity: ExpectedRule,
    /// Relative path of a source supplied by the case.
    pub file: RelativePath,
    /// Inclusive start offset in UTF-8 bytes.
    pub start: SourceByteOffset,
    /// Exclusive end offset in UTF-8 bytes.
    pub end: SourceByteOffset,
}

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

/// Whether the full inventory has been assessed or this is an interim authored subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkCoverage {
    /// Every default is implemented or linked to an assessed capability deferral.
    Accounted,
    /// Unassessed defaults remain explicit and the certificate covers only authored rules.
    Interim,
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

/// Source isolation contract at the benchmark boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectContextMode {
    /// Given text only; the root supplies no disk context.
    #[default]
    Isolated,
    /// Listed files and exact preloaded bytes, with a sealed disk dependency context.
    ListedDisk,
}

impl ProjectContextMode {
    /// Stable wire spelling included in invocation and certificate identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::ListedDisk => "listed-disk",
        }
    }
}

/// Authored auxiliary files and missing paths, all relative to an isolated snapshot boundary.
#[derive(Clone, Debug, Default)]
pub struct CaseContext {
    /// Run root within the snapshot; its directory components retain their spelling.
    pub logical_root: PathBuf,
    /// Auxiliary bytes supplied to both tools but never linted as requested sources.
    pub auxiliary_files: BTreeMap<PathBuf, Vec<u8>>,
    /// Paths explicitly sealed as absent.
    pub missing: Vec<PathBuf>,
}

/// Immutable expected state of a materialized context, independent of project lookup output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextManifest {
    /// Absolute snapshot boundary; dependencies must resolve inside it.
    pub boundary: PathBuf,
    /// Run root relative to the boundary, including its directory name.
    pub logical_root: PathBuf,
    /// Auxiliary paths and their content artifact hashes, relative to the boundary.
    pub auxiliary_files: BTreeMap<PathBuf, ArtifactDigest>,
    /// Explicit lint input bytes, relative to the same boundary, sealed independently of auxiliary files.
    pub source_files: BTreeMap<PathBuf, ArtifactDigest>,
    /// Paths explicitly sealed as missing, relative to the boundary.
    pub missing: Vec<PathBuf>,
}

/// Audited implementation inventory.
pub mod audit;
/// Opaque case-certified evidence.
pub mod certificate;

/// Language identifier for a shared corpus.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SuiteLanguage(
    /// Language identifier shared by the project corpus.
    pub String,
);
impl From<&str> for SuiteLanguage {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for SuiteLanguage {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for SuiteLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable benchmark project identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectName(
    /// Stable project name used in paths and observations.
    pub String,
);
impl From<&str> for ProjectName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for ProjectName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for ProjectName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Repository remote used to obtain an immutable revision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryUrl(
    /// Git remote used to fetch the pinned project.
    pub String,
);
impl From<&str> for RepositoryUrl {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for RepositoryUrl {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for RepositoryUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Full immutable Git commit identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectRevision(
    /// Immutable Git commit identifier.
    pub String,
);
impl From<&str> for ProjectRevision {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for ProjectRevision {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for ProjectRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Pinned reference tool identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolName(
    /// Reference tool name reported by its version command.
    pub String,
);
impl From<&str> for ToolName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for ToolName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Exact reference release identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolVersion(
    /// Exact required reference version.
    pub String,
);
impl From<&str> for ToolVersion {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for ToolVersion {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Expected executable SHA-256 digest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolDigest(
    /// Expected SHA256 hexadecimal digest of executable bytes.
    pub String,
);
impl From<&str> for ToolDigest {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for ToolDigest {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for ToolDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Assessed capability identifier for a default disposition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityName(
    /// Public capability identifier explaining an explicit deferral.
    pub String,
);
impl From<&str> for CapabilityName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for CapabilityName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable human-readable identity of an independent case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkCaseName(
    /// Unique human-readable case name within its rule.
    pub String,
);
impl From<&str> for BenchmarkCaseName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for BenchmarkCaseName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for BenchmarkCaseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Whether a worker observation starts fresh or reuses explicitly retained state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerState {
    /// Do not retain disk state between observations.
    Fresh,
    /// Reuse only this observation's explicitly selected state directory.
    Retained(PathBuf),
}

/// Stable digest of the complete requested artifact set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDigest(
    /// Framed artifact fingerprint, or raw executable hash when explicitly requested.
    pub String,
);
impl std::fmt::Display for ArtifactDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Number of independently executed correctness cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseCount(
    /// Number of independently authored cases checked.
    pub usize,
);

/// Number of cases checked against a reference tool without running a candidate.
///
/// This result is intentionally distinct from `CaseCount`: reference-only checking
/// validates fixture expectations but cannot certify candidate behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceCaseCount(
    /// Number of independently authored cases checked against the reference.
    pub usize,
);

/// Number of rules in a pinned coverage inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleCount(
    /// Number of rule identities in the audited inventory.
    pub usize,
);
impl std::fmt::Display for RuleCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::fmt::Display for CaseCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::fmt::Display for ReferenceCaseCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Complete per-rule verification status, independent of any tool's rule count.
pub mod inventory;

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

/// Whether this concrete contestant implements the declared configuration contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleConfigurationSupport {
    /// This adapter must refuse a configured request before starting a process.
    Unsupported,
    /// Exact disabled-file pairs and external prefixes are projected without a second matcher.
    Explicit,
}
