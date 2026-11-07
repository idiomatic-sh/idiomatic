//! The types of the `inventory` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.
use ryo_benchmark_contracts::domain::documents::types::{finding::Finding, observation_document::{
        ContextMode, ExitStatus, FileMetadataDigest, ObservationDocument, ProjectInputsDigest,
        RuleCohort, RuleContextRecord, RuleObservationsRecord,
    }, reference_discrepancy::ReferenceDiscrepancy, text::{CanonicalJson, CaseName, PathText, ReasonText, RuleName, Sha256Text, YesNo, CanonicalJson as _}};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, CaseCount, FileObservation};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// The name of the provider a registration comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderName(pub String);

/// The key of one pin of the provider's source or worker.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdentityKey(pub String);

/// The logical name of one query module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegisteredModuleName(pub String);

/// A generated registration, never a second manually maintained rule roster.
#[derive(Clone, Debug, PartialEq)]
pub struct Registration {
    /// Explicit execution authority or receipt-only interpretation; never inferred.
    pub authority: RegistrationAuthority,
    /// Provider identity for report consumers.
    pub provider: ProviderName,
    /// Complete defaults read from the provider's pinned benchmark profile.
    pub defaults: Vec<RuleName>,
    /// Implementations or explicit registration failures for those defaults.
    pub rules: Vec<RegisteredRule>,
    /// Source/worker pin metadata supplied by the provider adapter, as canonical JSON text.
    pub identity: BTreeMap<IdentityKey, CanonicalJson>,
}

/// The exact tested query definition of a registered rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleDefinition {
    /// The definition is pinned.
    Pinned(FilePin),
    /// No definition is registered.
    Unpinned,
}

/// Whether a registered rule has an implementation to invoke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleCommand {
    /// An implementation can be invoked.
    Registered(Command),
    /// No implementation can be invoked.
    Unregistered,
}

/// Why a registered rule cannot run, or why a row failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleReason {
    /// The explicit missing or configuration reason, never a skipped success.
    Stated(ReasonText),
    /// No reason is stated.
    Unstated,
}

/// The tool-specific source identity of a registered rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleInstance {
    /// The identity, as canonical JSON text, preserved without interpretation by the runner.
    Declared(CanonicalJson),
    /// No identity is declared.
    Undeclared,
}

/// One ordinary native verifier invocation with exact registered case identities.
#[derive(Clone, Debug, PartialEq)]
pub struct RegisteredRule {
    /// Exact tested query definition, independently linked to measured code.
    pub definition: RuleDefinition,
    /// Named helper definitions in this registered context.
    pub modules: BTreeMap<RegisteredModuleName, FilePin>,
    /// Exact regular-file context, preserving each registered profile namespace.
    pub artifacts: BTreeMap<PathBuf, Sha256Text>,
    /// Pinned default identifier.
    pub rule_id: RuleName,
    /// Whether an implementation can be invoked.
    pub command: RuleCommand,
    /// Explicit missing/configuration reason.
    pub reason: RuleReason,
    /// Complete independently authored case identities, including evidence pins.
    pub cases: Vec<CaseDocument>,
    /// Tool-specific source identity.
    pub instance: RuleInstance,
}

/// The shared worker's native case-verification operation. Executable and reference
/// come exclusively from registration authority; argv/cwd/environment are not authorable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Execute the existing shared verify-cases protocol over this exact pinned context.
    CaseVerification { context: PathBuf },
}

/// Whether a result row passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowStatus {
    /// The rule verified.
    Pass,
    /// The rule failed, was missing or was not run.
    Fail,
}

/// The failure explanation of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowReason {
    /// Failure explanation, including missing and unrun.
    Failed(ReasonText),
    /// The row passed.
    Passed,
}

/// The original per-rule source identity of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowInstance {
    /// The identity, as canonical JSON text.
    Carried(CanonicalJson),
    /// The rule carries no identity.
    Absent,
}

/// The native receipt identity of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptDigest {
    /// Actual verification wrote a receipt with this digest.
    Recorded(Sha256Text),
    /// No receipt exists yet.
    Unrecorded,
}

/// Public summary row: only PASS or FAIL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Rule identity from the pinned inventory.
    pub rule_id: RuleName,
    /// PASS or FAIL only.
    pub status: RowStatus,
    /// Failure explanation.
    pub reason: RowReason,
    /// Original per-rule source identity.
    pub instance: RowInstance,
    /// Native receipt identity.
    pub case_results_sha256: ReceiptDigest,
    /// Complete case pairs.
    pub case_count: CaseCount,
    /// Exact raw case pairs.
    pub raw_exact: CaseCount,
    /// Independently evidenced reference differences.
    pub documented_discrepancies: CaseCount,
    /// Declared-contract passes that are not raw report equality or reference discrepancies.
    pub contract_only: CaseCount,
}

/// Complete or running state of a summary, not a third per-rule status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    /// Rules are still being verified.
    Running,
    /// Every rule has a row.
    Complete,
    /// The run was refused before every rule had a row.
    Failed,
}

/// Atomic public inventory snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Provider's stable name.
    pub provider: ProviderName,
    /// Complete/running state.
    pub run_state: RunState,
    /// Source and executable pins, as canonical JSON text.
    pub identity: BTreeMap<IdentityKey, CanonicalJson>,
    /// How many rules passed; PASS plus FAIL always equals the declared registry length.
    pub passed: CaseCount,
    /// How many rules failed.
    pub failed: CaseCount,
    /// Every pinned rule, including missing implementations.
    pub rules: Vec<Row>,
}

/// Native observations actually present after one child invocation.
#[derive(Clone, Debug, PartialEq)]
pub struct CapturedObservation {
    /// Exact receipt-scoped raw process artifacts, keyed by safe relative path.
    pub artifacts: BTreeMap<PathBuf, CapturedBytes>,
    /// Absent receipts are not fabricated as successful records.
    pub records: Vec<ObservationDocument>,
    /// The raw invocation record of every captured process, keyed like its artifact.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
    /// Absent until a native case receipt exists.
    pub sha256: ReceiptDigest,
}

/// Receipt interpretation cannot authorize execution or benchmark certification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistrationAuthority {
    /// Parse existing receipts only; execution explicitly refuses this variant.
    ReceiptOnly,
    /// Registration failed before native execution; every row remains a failure.
    Unavailable { reason: ReasonText },
    /// Pinned root inventory and both actual native executable identities.
    Execution {
        profile: PathBuf,
        profile_sha256: Sha256Text,
        reference: ExecutablePin,
        worker: ExecutablePin,
    },
}

/// An explicit executable location and immutable byte identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutablePin {
    pub path: PathBuf,
    pub sha256: Sha256Text,
}

/// Exact raw content of one explicit case-context file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePin {
    pub path: PathBuf,
    pub sha256: Sha256Text,
}

/// Every regular file of a registered context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextFiles {
    /// The absolute path of each file.
    pub files: BTreeSet<PathBuf>,
}

/// Whether a native case receipt exists beside the captured streams.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptFile {
    /// The verifier wrote no receipt.
    Absent,
    /// The verifier wrote this receipt.
    Present(PathBuf),
}

/// A documented difference from the reference tool, or its absence.
#[derive(Clone, Debug, PartialEq)]
pub enum CaseReference {
    /// The case documents no difference.
    Agrees,
    /// The case documents this difference.
    Differs(ReferenceDiscrepancy),
}

/// The listed context a case was authored against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseContextIdentity {
    /// The mode is isolated: no listed context is written.
    Isolated,
    /// The mode is listed-disk: the context the case was authored against.
    Listed(ContextIdentity),
}

/// The evidence a caller requires of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectationContract {
    /// The authoring never writes it: the evidence is diagnostics.
    Unwritten,
    /// The document states the evidence.
    Declared(ComparisonContract),
}

/// The declared execute bit of each file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileMetadata {
    /// The case declares no file metadata.
    Undeclared,
    /// The case declares the execute bit of each file.
    Declared(BTreeMap<PathText, FileExecutable>),
}

/// The project search roots a case or profile declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredScope {
    /// No roots are declared: namespace membership stays unknown.
    Undeclared,
    /// These roots are declared; an empty list is a declared empty scope.
    Roots(Vec<PathText>),
}

/// The project entries a case declares, written with the project search roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectInventory {
    /// No entries are declared.
    Undeclared,
    /// These entries are declared.
    Declared(ProjectInventoryDocument),
}

/// The listed context a case was authored against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextIdentity {
    /// How the context is staged.
    pub mode: ContextMode,
    /// The root the sources are logically under.
    pub logical_root: PathText,
    /// Framed digest of the declared context bytes.
    pub manifest_hash: Sha256Text,
    /// Every declared present path.
    pub present_paths: Vec<PathText>,
    /// Every declared missing path.
    pub missing_paths: Vec<PathText>,
}

/// How complete a project inventory is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InventoryCompleteness {
    /// Entries listed are known; others are not.
    #[default]
    Partial,
    /// Every entry within the declared roots is listed.
    CompleteWithinDeclaredScope,
}

/// What an inventory entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryState {
    /// A regular file.
    Regular,
    /// A directory.
    Directory,
    /// Another kind of entry.
    Other,
    /// Known to be absent.
    Missing,
    /// Not observed.
    Unknown,
}

/// One project entry a case declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectEntry {
    /// The entry's project-relative path.
    pub path: PathText,
    /// What the entry is.
    pub state: EntryState,
}

/// The project entries a case declares for namespace resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectInventoryDocument {
    /// How complete the listing is.
    pub completeness: InventoryCompleteness,
    /// The listed entries.
    pub entries: Vec<ProjectEntry>,
}

/// A registered case: identity, expectations and every declared input.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseDocument {
    /// The rule the case belongs to.
    pub rule: RuleName,
    /// The case's name within its rule.
    pub name: CaseName,
    /// Digest of the authored case text.
    pub case_sha256: Sha256Text,
    /// Digest of the case's sources.
    pub source_sha256: Sha256Text,
    /// The case's source files.
    pub files: Vec<PathText>,
    /// How much of the report the case compares.
    pub comparison: CaseComparison,
    /// The diagnostics the candidate must report.
    pub expected: Vec<Finding>,
    /// How the sources are staged.
    pub project_context_mode: ContextMode,
    /// The listed context the case was authored against.
    pub context_identity: CaseContextIdentity,
    /// A documented difference from the reference tool: written when the case documents one.
    pub reference_discrepancy: CaseReference,
    /// The evidence a caller requires.
    pub expectation_contract: ExpectationContract,
    /// The prepared rule context.
    pub rule_context: RuleContextRecord,
    /// The declared rule observations.
    pub rule_observations: RuleObservationsRecord,
    /// The explicit rule cohort.
    pub selected_rules: RuleCohort,
    /// The file observation the profile declares.
    pub file_observation: FileObservation,
    /// The declared execute bit of each file.
    pub file_metadata: FileMetadata,
    /// Digest of the declared file metadata: written with the file metadata.
    pub file_metadata_sha256: FileMetadataDigest,
    /// The project search roots.
    pub project_search_roots: DeclaredScope,
    /// The declared project entries.
    pub given_project_inventory: ProjectInventory,
    /// Digest of the declared project inputs.
    pub project_inputs_sha256: ProjectInputsDigest,
}

/// One argument of the captured command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentText(pub String);

/// The name of one captured environment override.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvironmentName(pub String);

/// The value of one captured environment override.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentValue(pub String);

/// The terminating signal of a captured process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// The process was not signalled.
    Absent,
    /// The process was ended by this signal.
    Number(ExitStatus),
}

/// An identity a record may leave out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pin {
    /// The record binds no such identity.
    Absent,
    /// The record binds this identity.
    Sha256(Sha256Text),
}

/// The normalized settings a capture is bound to, with the three parts raw metadata re-derives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedSettings {
    /// The normalized command line.
    pub arguments: Vec<ArgumentText>,
    /// The normalized environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The normalized working directory.
    pub directory: PathText,
    /// The whole settings in canonical sorted-key form.
    pub canonical: CanonicalJson,
}

/// A capture whose process identity was recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteInvocation {
    /// The directory the request ran under.
    pub actual_request_root: PathText,
    /// The sources the request read.
    pub files: Vec<PathText>,
    /// The executable that ran.
    pub executable_sha256: Sha256Text,
    /// The normalized settings digest.
    pub settings_sha256: Sha256Text,
    /// The normalized settings.
    pub normalized_settings: NormalizedSettings,
    /// The pinned profile.
    pub profile_sha256: Sha256Text,
    /// The tool's name.
    pub tool: ReasonText,
    /// The tool's version.
    pub version: ReasonText,
    /// The state boundary of the request.
    pub state: PathText,
    /// The raw command line.
    pub arguments: Vec<ArgumentText>,
    /// The raw environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The raw working directory.
    pub directory: PathText,
    /// The numeric exit status.
    pub exit: ExitStatus,
    /// The terminating signal.
    pub signal: Signal,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: YesNo,
    /// The case sources.
    pub source_sha256: Pin,
    /// The declared file metadata.
    pub file_metadata_sha256: Pin,
}

/// A capture whose process identity could not be recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedInvocation {
    /// The directory the request ran under.
    pub actual_request_root: PathText,
    /// The sources the request read.
    pub files: Vec<PathText>,
    /// The raw command line.
    pub arguments: Vec<ArgumentText>,
    /// The raw environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The raw working directory.
    pub directory: PathText,
    /// The numeric exit status.
    pub exit: ExitStatus,
    /// The terminating signal.
    pub signal: Signal,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: YesNo,
    /// Why the identity could not be recorded.
    pub metadata_error: ReasonText,
    /// The case sources.
    pub source_sha256: Pin,
    /// The declared file metadata.
    pub file_metadata_sha256: Pin,
}

/// One raw invocation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvocationDocument {
    /// The process identity was recorded.
    Complete(CompleteInvocation),
    /// The process identity could not be recorded.
    Failed(FailedInvocation),
}

/// The raw bytes a receipt points at and the invocation records decoded from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawArtifacts {
    /// Exact receipt-scoped raw process artifacts, keyed by safe relative path.
    pub bytes: BTreeMap<PathBuf, CapturedBytes>,
    /// The raw invocation record of every captured process, keyed like its artifact.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
}
