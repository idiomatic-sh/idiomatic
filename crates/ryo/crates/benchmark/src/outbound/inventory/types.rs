//! Tool-neutral wire data; adapters own format and filesystem details.
use crate::outbound::inventory::constants;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// A generated registration, never a second manually maintained rule roster.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    pub identity: BTreeMap<String, serde_json::Value>,
}
/// One ordinary native verifier invocation with exact registered case identities.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    pub cases: Vec<serde_json::Value>,
    /// Tool-specific source identity, preserved without interpretation by the runner.
    pub instance: Option<serde_json::Value>,
}
/// Shared verifier argv contract: every registered verifier accepts this explicit receipt destination.
pub const CASE_RECEIPTS_ARGUMENT: &str = constants::CASE_RECEIPTS;

/// The shared worker's native case-verification operation. Executable and reference
/// come exclusively from registration authority; argv/cwd/environment are not authorable.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
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
    pub records: Vec<serde_json::Value>,
    /// Absent until a native case receipt exists.
    pub sha256: Option<String>,
}

/// Receipt interpretation cannot authorize execution or benchmark certification.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
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
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
