//! The types of the `inventory` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::{CanonicalJson, ReasonText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::workloads::types::CaseCount;
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

/// The raw bytes a receipt points at and the invocation records decoded from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawArtifacts {
    /// Exact receipt-scoped raw process artifacts, keyed by safe relative path.
    pub bytes: BTreeMap<PathBuf, CapturedBytes>,
    /// The raw invocation record of every captured process, keyed like its artifact.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
}
