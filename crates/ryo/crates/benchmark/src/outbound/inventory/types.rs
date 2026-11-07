//! Tool-neutral wire data; adapters own format and filesystem details.
pub mod case_document;

use self::case_document::CaseDocument;
use crate::outbound::suite::types::{
    observation_document::{ExitStatus, ObservationDocument},
    text::{CanonicalJson, PathText, ReasonText, Sha256Text, YesNo},
};
use crate::outbound::inventory::constants;
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

/// A generated registration, never a second manually maintained rule roster.
#[derive(Clone, Debug)]
pub struct Registration {
    /// Explicit execution authority or receipt-only interpretation; never inferred.
    pub authority: RegistrationAuthority,
    /// Provider identity for report consumers.
    pub provider: String,
    /// Complete defaults read from the provider's pinned benchmark profile.
    pub defaults: Vec<String>,
    /// Implementations or explicit registration failures for those defaults.
    pub rules: Vec<RegisteredRule>,
    /// Source/worker pin metadata supplied by the provider adapter.
    pub identity: BTreeMap<String, serde_json::Value>,
}
/// One ordinary native verifier invocation with exact registered case identities.
#[derive(Clone, Debug)]
pub struct RegisteredRule {
    /// Exact tested query definition, independently linked to measured code.
    pub definition: Option<FilePin>,
    /// Named helper definitions in this registered context.
    pub modules: BTreeMap<String, FilePin>,
    /// Exact regular-file context, preserving each registered profile namespace.
    pub artifacts: BTreeMap<PathBuf, String>,
    /// Pinned default identifier.
    pub rule_id: String,
    /// Absent when no implementation can be invoked.
    pub command: Option<Command>,
    /// Explicit missing/configuration reason, never a skipped success.
    pub reason: Option<String>,
    /// Complete independently authored case identities, including evidence pins.
    pub cases: Vec<CaseDocument>,
    /// Tool-specific source identity, preserved without interpretation by the runner.
    pub instance: Option<serde_json::Value>,
}
/// Shared verifier argv contract: every registered verifier accepts this explicit receipt destination.
pub const CASE_RECEIPTS_ARGUMENT: &str = constants::CASE_RECEIPTS;

/// The shared worker's native case-verification operation. Executable and reference
/// come exclusively from registration authority; argv/cwd/environment are not authorable.
#[derive(Clone, Debug)]
pub enum Command {
    /// Execute the existing shared verify-cases protocol over this exact pinned context.
    CaseVerification { context: PathBuf },
}
/// Public summary row: only PASS or FAIL.
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    /// Rule identity from the pinned inventory.
    pub rule_id: String,
    /// PASS or FAIL only.
    pub status: &'static str,
    /// Failure explanation, including missing and unrun.
    pub reason: Option<String>,
    /// Original per-rule source identity.
    pub instance: Option<serde_json::Value>,
    /// Native receipt identity, absent until actual verification.
    pub case_results_sha256: Option<String>,
    /// Complete case pairs.
    pub case_count: usize,
    /// Exact raw case pairs.
    pub raw_exact: usize,
    /// Independently evidenced reference differences.
    pub documented_discrepancies: usize,
    /// Declared-contract passes that are not raw report equality or reference discrepancies.
    pub contract_only: usize,
}
/// Atomic public inventory snapshot.
#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    /// Shared schema, with no provider-specific rule count.
    pub schema: &'static str,
    /// Provider's stable name.
    pub provider: String,
    /// Complete/running state, not a third per-rule status.
    pub run_state: &'static str,
    /// Source and executable pins.
    pub identity: BTreeMap<String, serde_json::Value>,
    /// PASS plus FAIL always equals the declared registry length.
    pub counts: BTreeMap<&'static str, usize>,
    /// Every pinned rule, including missing implementations.
    pub rules: Vec<Row>,
}

/// Native observations actually present after one child invocation.
#[derive(Clone, Debug)]
pub struct CapturedObservation {
    /// Exact receipt-scoped raw process artifacts, keyed by safe relative path.
    pub artifacts: BTreeMap<PathBuf, Vec<u8>>,
    /// Absent receipts are not fabricated as successful records.
    pub records: Vec<ObservationDocument>,
    /// The raw invocation record of every captured process, keyed like its artifact.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
    /// Absent until a native case receipt exists.
    pub sha256: Option<String>,
}

/// Receipt interpretation cannot authorize execution or benchmark certification.
#[derive(Clone, Debug)]
pub enum RegistrationAuthority {
    /// Parse existing receipts only; execution explicitly refuses this variant.
    ReceiptOnly,
    /// Registration failed before native execution; every row remains a failure.
    Unavailable { reason: String },
    /// Pinned root inventory and both actual native executable identities.
    Execution {
        profile: PathBuf,
        profile_sha256: String,
        reference: ExecutablePin,
        worker: ExecutablePin,
    },
}
/// An explicit executable location and immutable byte identity.
#[derive(Clone, Debug)]
pub struct ExecutablePin {
    pub path: PathBuf,
    pub sha256: String,
}
/// Facts read from the canonical profile and actual executable bytes.
#[derive(Clone, Debug)]
pub struct CanonicalFacts {
    pub profile_sha256: String,
    pub reference_sha256: String,
    pub worker_sha256: String,
    pub defaults: Vec<String>,
    pub deferred: Vec<String>,
    pub pending: Vec<String>,
    pub approved_reference: Vec<String>,
}

/// Exact raw content of one explicit case-context file.
#[derive(Clone, Debug)]
pub struct FilePin {
    pub path: PathBuf,
    pub sha256: String,
}

/// Module declarations read from a fully verified registered context profile.
#[derive(Clone, Debug)]
pub struct ContextFacts {
    /// Complete logical-name to profile-relative-path map; empty means declared empty.
    pub modules: BTreeMap<String, PathBuf>,
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
    pub bytes: BTreeMap<PathBuf, Vec<u8>>,
    /// The raw invocation record of every captured process, keyed like its artifact.
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
}
