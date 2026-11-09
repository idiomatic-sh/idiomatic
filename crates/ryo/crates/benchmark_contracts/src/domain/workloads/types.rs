//! Immutable benchmark workload definitions and external artifact observations.
use self::benchmark_case_name::value::BenchmarkCaseName;
use self::capability_name::value::CapabilityName;
use self::discrepancy_evidence::value::DiscrepancyEvidence;
use self::project_name::value::ProjectName;
use self::project_revision::value::ProjectRevision;
use self::repository_url::value::RepositoryUrl;
use self::suite_language::value::SuiteLanguage;
use self::tool_digest::value::ToolDigest;
use self::tool_name::value::ToolName;
use self::tool_version::value::ToolVersion;
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use crate::domain::context::types::CaseContext;
use crate::domain::context::types::ContextManifest;
use crate::domain::context::types::ObservationSelectionPolicy;
use crate::domain::context::types::RuleConfiguration;
use crate::domain::context::types::RuleContextDeclaration;
use crate::domain::context::types::project_context_mode::value::ProjectContextMode;
use crate::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use crate::domain::reports::types::SourceByteOffset;
use crate::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, FileMetadataDeclaration, InputDeclaration,
    ParserSelection, PerformanceOptions, ProviderSelection, QueryModuleName, RuleLabel, RunRequest,
    SearchRoots,
};
use ryo::linter::RelativePath;
use ryo::linter::RuleId;
use ryo::linter::SourceText;
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

/// The independently pinned reference executable and its shipped defaults.
#[derive(Clone, Debug)]
pub struct Tool {
    /// Explicit preparation policy for profile-default observation declarations.
    pub rule_observation_selection: ObservationSelectionPolicy,
    /// Explicit executable support definitions; never default selection or case-owner credit.
    pub supporting_rules: Vec<RuleId>,
    /// Default authored named inputs; case overrides are explicit and source-bound.
    pub rule_observations: RuleObservationDeclarations,
    /// Optional explicitly declared request-scope rule configuration inputs.
    pub rule_context: RuleContextProfile,
    /// File observation this correctness profile requires for every declared source.
    pub file_observation: FileObservation,
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
    pub query_modules: BTreeMap<QueryModuleName, PathBuf>,
    /// Pinned phase policy for authored candidate rules; absent entries retain legacy Always.
    pub rule_applicability: BTreeMap<RuleLabel, ApplicabilityPolicy>,
    /// Optional parser configuration that must be shared by reference and candidate.
    pub parser_configuration: ParserSelection,
    /// Explicit source provider selected by the request edge, if any.
    pub source_language: ProviderSelection,
    /// Ordered project import roots; absence keeps namespace membership unknown.
    pub project_search_roots: SearchRoots,
}

/// The request-scope rule configuration a profile declares.
#[derive(Clone, Debug)]
pub enum RuleContextProfile {
    /// The profile declares no request-scope configuration.
    Undeclared,
    /// The profile declares this configuration.
    Declared(RuleContextDeclaration),
}

/// Generic input observation required by a profile's comparison contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileObservation {
    /// The profile declares no file observation.
    Undeclared,
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
    pub selected_rules: CohortSelection,
    /// Explicit named input declarations for the configured consumer queries.
    pub rule_observations: InputDeclaration,
    /// Explicit complete override of the profile rule configuration.
    pub rule_context: ConfigurationOverride,
    /// Optional case-specific ordered import roots; an empty declared list is meaningful.
    pub project_search_roots: SearchRoots,
    /// Explicit host entry observations for a Given source corpus.
    pub given_project_inventory: InventoryDeclaration,
    /// Explicit primary inventory, including selected paths sealed absent by the context.
    /// `Legacy` preserves the inventory of exactly the source-text map's keys.
    pub requested_files: RequestedFiles,
    /// Explicit per-file POSIX executable bits; `Undeclared` preserves legacy byte-only identity.
    pub file_metadata: FileMetadataDeclaration,
    /// Explicit logical root and auxiliary context, excluded from the lint file inventory.
    pub context: ContextDeclaration,
    /// Whether all diagnostics or only the enclosing rule are asserted.
    pub comparison: CaseComparison,
    /// Human-readable identity of this correctness case.
    pub name: BenchmarkCaseName,
    /// Relative source paths and their complete UTF-8 contents.
    pub files: BTreeMap<RelativePath, SourceText>,
    /// Expected finding multiset; an empty list is a negative case.
    pub expected: Vec<Expected>,
    /// Optional pinned reference behavior that intentionally differs from the candidate contract.
    pub reference_discrepancy: ReferenceBehavior,
}

/// Whether a case's reference tool behaves as the candidate contract intends.
#[derive(Clone, Debug)]
pub enum ReferenceBehavior {
    /// The reference agrees with the intended behavior.
    Agrees,
    /// The reference differs, with documented evidence.
    Differs(ReferenceDiscrepancy),
}

/// The case-specific override of the profile rule configuration.
#[derive(Clone, Debug)]
pub enum ConfigurationOverride {
    /// The profile configuration applies unchanged.
    Inherited,
    /// This configuration replaces the profile's completely.
    Overridden(RuleConfiguration),
}

/// Host entry observations of a Given source corpus.
#[derive(Clone, Debug)]
pub enum InventoryDeclaration {
    /// No entries are declared.
    Undeclared,
    /// These entries are declared.
    Declared(ryo::linter::GivenProjectInventory),
}

/// The primary inventory of a case.
#[derive(Clone, Debug)]
pub enum RequestedFiles {
    /// The inventory is exactly the source-text map's keys.
    Legacy,
    /// This inventory, including paths sealed absent by the context.
    Explicit(Vec<RelativePath>),
}

/// The logical root and auxiliary files of a case.
#[derive(Clone, Debug)]
pub enum ContextDeclaration {
    /// The case supplies no context.
    Undeclared,
    /// This context is supplied.
    Declared(CaseContext),
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
    pub reason: DiscrepancyReason,
}

/// Why a reference discrepancy is accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscrepancyReason(
    /// The explanation of the intended behavior and reference disagreement.
    pub String,
);

/// Immutable identities required before admitting a reference discrepancy.
#[derive(Clone, Debug)]
pub struct ReferenceDiscrepancyIdentity {
    /// Separate declared-metadata identity; absent only for legacy unspecified observations.
    pub file_metadata_sha256: MetadataDigestDeclaration,
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

/// The digest of the declared file metadata of a discrepancy's source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataDigestDeclaration {
    /// Absent only for legacy unspecified observations.
    Undeclared,
    /// The case declares file metadata, under this digest.
    Declared(ArtifactDigest),
}

/// The authority asserted by a documented two-sided difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscrepancyBasis {
    /// Independent language meaning contradicts the reference output.
    LanguageDefect,
    /// A documented public tool policy differs from the pinned reference tool.
    ToolContract,
}

/// Materialized reference expectation with independently validated evidence.
#[derive(Clone, Debug)]
pub struct PreparedReferenceDiscrepancy {
    /// Language defect or deliberately different public tool policy.
    pub basis: DiscrepancyBasis,
    /// Exact reference report; distinct from the candidate's intended expected report.
    pub expected: crate::domain::process::types::ReferenceExpectation,
    /// Immutable execution and output identities.
    pub identity: ReferenceDiscrepancyIdentity,
    /// Independent evidence retained for reporting and stale-artifact refusal.
    pub evidence: Vec<DiscrepancyEvidence>,
    /// Human-readable explanation retained beside raw parity mismatches.
    pub reason: DiscrepancyReason,
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
    pub expected: crate::domain::reports::types::RunReport,
    /// Optional separate reference expectation; never changes the intended candidate output.
    pub reference_discrepancy: PreparedReferenceBehavior,
}

/// Whether a prepared case's reference tool differs from the intended behavior.
#[derive(Clone, Debug)]
pub enum PreparedReferenceBehavior {
    /// The reference agrees with the intended behavior.
    Agrees,
    /// The reference differs, with validated evidence.
    Differs(PreparedReferenceDiscrepancy),
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

/// Whether this concrete contestant implements the declared configuration contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleConfigurationSupport {
    /// This adapter must refuse a configured request before starting a process.
    Unsupported,
    /// Exact disabled-file pairs and external prefixes are projected without a second matcher.
    Explicit,
}

/// Artifact staging keeps independent certification evidence separate from execution definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagePurpose {
    /// One registered profile's cases and evidence retain their own relative namespace.
    Certification,
    /// Complete measured queries/modules; case evidence is certified in registered contexts.
    Execution,
}

pub mod suite_language;

pub mod project_name;

pub mod repository_url;

pub mod project_revision;

pub mod tool_name;

pub mod tool_version;

pub mod tool_digest;

pub mod capability_name;

pub mod benchmark_case_name;

pub mod case_count;

pub mod reference_case_count;

pub mod rule_count;

pub mod discrepancy_evidence;
