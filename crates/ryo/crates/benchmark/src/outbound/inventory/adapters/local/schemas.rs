//! The wire shapes of the registration and of the native receipt rows. Unknown fields are refused:
//! the documents are a contract, and the registration authoring is its source of truth.
use super::constants;
use crate::outbound::inventory::types::{
    ArgumentText, CaseContextIdentity, CaseDocument, CaseReference, Command, CompleteInvocation,
    ContextIdentity, DeclaredScope, EntryState, EnvironmentName, EnvironmentValue,
    ExecutablePin, ExpectationContract, FailedInvocation, FileMetadata, FilePin,
    IdentityKey, InventoryCompleteness, InvocationDocument, RegisteredModuleName, NormalizedSettings, Pin,
    ProjectEntry, ProjectInventory, ProjectInventoryDocument, ProviderName, ReceiptDigest,
    RegisteredRule, Registration, RegistrationAuthority, RowInstance, RowReason, RowStatus,
    RuleCommand, RuleDefinition, RuleInstance, RuleReason, RunState, Row, Signal, Summary,
};
use crate::outbound::suite::types::{process::ProcessFailureWire};
use ryo_benchmark_contracts::domain::documents::types::{finding::{FindingOffset, Finding}, observation_document::{
        ContextMode, ExitStatus, FileMetadataDigest, ProjectInputsDigest, ReferenceExpectation, RuleCohort,
        RuleContextRecord, RuleObservationsRecord,
    }, reference_discrepancy::{
        DiscrepancyIdentity, ObservedExecutable, ReferenceDiscrepancy,
    }, text::{CanonicalJson, CaseName, PathText, ReasonText, RuleName, Sha256Text, YesNo}};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, DiscrepancyBasis, FileObservation};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

fn sha(value: Option<String>) -> Option<Sha256Text> {
    value.map(Sha256Text)
}
fn path(value: String) -> PathText {
    PathText(value)
}
fn canonical(value: Option<serde_json::Value>) -> Option<CanonicalJson> {
    value.map(|value| CanonicalJson(value.to_string()))
}
fn findings(rows: Vec<FindingSchema>) -> Vec<Finding> {
    rows.into_iter().map(Finding::from).collect()
}

/// The generated registration of one provider's rules.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RegistrationSchema {
    authority: AuthoritySchema,
    provider: String,
    defaults: Vec<String>,
    rules: Vec<RegisteredRuleSchema>,
    #[serde(default)]
    identity: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredRuleSchema {
    definition: Option<FilePinSchema>,
    modules: BTreeMap<String, FilePinSchema>,
    artifacts: BTreeMap<PathBuf, String>,
    rule_id: String,
    command: Option<CommandSchema>,
    reason: Option<String>,
    #[serde(default)]
    cases: Vec<CaseSchema>,
    instance: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePinSchema {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutablePinSchema {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum CommandSchema {
    CaseVerification { context: PathBuf },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum AuthoritySchema {
    ReceiptOnly,
    Unavailable {
        reason: String,
    },
    Execution {
        profile: PathBuf,
        profile_sha256: String,
        reference: ExecutablePinSchema,
        worker: ExecutablePinSchema,
    },
}

impl From<RegistrationSchema> for Registration {
    fn from(wire: RegistrationSchema) -> Self {
        Self {
            authority: match wire.authority {
                AuthoritySchema::ReceiptOnly => RegistrationAuthority::ReceiptOnly,
                AuthoritySchema::Unavailable { reason } => RegistrationAuthority::Unavailable {
                    reason: ReasonText(reason),
                },
                AuthoritySchema::Execution {
                    profile,
                    profile_sha256,
                    reference,
                    worker,
                } => RegistrationAuthority::Execution {
                    profile,
                    profile_sha256: Sha256Text(profile_sha256),
                    reference: ExecutablePin {
                        path: reference.path,
                        sha256: Sha256Text(reference.sha256),
                    },
                    worker: ExecutablePin {
                        path: worker.path,
                        sha256: Sha256Text(worker.sha256),
                    },
                },
            },
            provider: ProviderName(wire.provider),
            defaults: wire.defaults.into_iter().map(RuleName).collect(),
            rules: wire.rules.into_iter().map(RegisteredRule::from).collect(),
            identity: wire
                .identity
                .into_iter()
                .map(|(key, value)| (IdentityKey(key), CanonicalJson(value.to_string())))
                .collect(),
        }
    }
}

impl From<RegisteredRuleSchema> for RegisteredRule {
    fn from(wire: RegisteredRuleSchema) -> Self {
        let pin = |pin: FilePinSchema| FilePin {
            path: pin.path,
            sha256: Sha256Text(pin.sha256),
        };
        Self {
            definition: wire
                .definition
                .map_or(RuleDefinition::Unpinned, |value| {
                    RuleDefinition::Pinned(pin(value))
                }),
            modules: wire
                .modules
                .into_iter()
                .map(|(name, value)| (RegisteredModuleName(name), pin(value)))
                .collect(),
            artifacts: wire
                .artifacts
                .into_iter()
                .map(|(path, digest)| (path, Sha256Text(digest)))
                .collect(),
            rule_id: RuleName(wire.rule_id),
            command: wire.command.map_or(RuleCommand::Unregistered, |command| {
                RuleCommand::Registered(match command {
                    CommandSchema::CaseVerification { context } => {
                        Command::CaseVerification { context }
                    }
                })
            }),
            reason: wire
                .reason
                .map_or(RuleReason::Unstated, |reason| RuleReason::Stated(ReasonText(reason))),
            cases: wire.cases.into_iter().map(CaseDocument::from).collect(),
            instance: wire.instance.map_or(RuleInstance::Undeclared, |value| {
                RuleInstance::Declared(CanonicalJson(value.to_string()))
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingSchema {
    rule: String,
    file: String,
    start: u64,
    end: u64,
}

impl From<FindingSchema> for Finding {
    fn from(wire: FindingSchema) -> Self {
        Self {
            rule: RuleName(wire.rule),
            file: PathText(wire.file),
            start: FindingOffset(wire.start),
            end: FindingOffset(wire.end),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComparisonSchema {
    Rule,
    Full,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContextModeSchema {
    Isolated,
    ListedDisk,
}

impl From<ContextModeSchema> for ContextMode {
    fn from(wire: ContextModeSchema) -> Self {
        match wire {
            ContextModeSchema::Isolated => Self::Isolated,
            ContextModeSchema::ListedDisk => Self::ListedDisk,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContractSchema {
    Diagnostics,
    CompleteReport,
}

impl From<ContractSchema> for ComparisonContract {
    fn from(wire: ContractSchema) -> Self {
        match wire {
            ContractSchema::Diagnostics => Self::Diagnostics,
            ContractSchema::CompleteReport => Self::CompleteReport,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FileObservationSchema {
    PosixExecutable,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextIdentitySchema {
    mode: ContextModeSchema,
    logical_root: String,
    manifest_hash: String,
    present_paths: Vec<String>,
    missing_paths: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntrySchema {
    path: String,
    state: EntryStateSchema,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventorySchema {
    #[serde(default)]
    completeness: CompletenessSchema,
    #[serde(default)]
    entries: Vec<EntrySchema>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BasisSchema {
    LanguageDefect,
    ToolContractDifference,
}

impl From<BasisSchema> for DiscrepancyBasis {
    fn from(wire: BasisSchema) -> Self {
        match wire {
            BasisSchema::LanguageDefect => Self::LanguageDefect,
            BasisSchema::ToolContractDifference => Self::ToolContract,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct IdentitySchema {
    tool: String,
    version: String,
    observed_executable_sha256: Option<String>,
    profile_sha256: String,
    settings_sha256: String,
    source_sha256: String,
    reference_output_sha256: String,
    intended_output_sha256: String,
    file_metadata_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscrepancySchema {
    basis: Option<BasisSchema>,
    identity: IdentitySchema,
    approved_executable_sha256: Vec<String>,
    reference_expected: Option<Vec<FindingSchema>>,
    intended_expected: Vec<FindingSchema>,
    reason: String,
    process: Option<ProcessFailureWire>,
}

impl From<DiscrepancySchema> for ReferenceDiscrepancy {
    fn from(wire: DiscrepancySchema) -> Self {
        let identity = wire.identity;
        Self {
            basis: wire
                .basis
                .map_or(DiscrepancyBasis::LanguageDefect, Into::into),
            identity: DiscrepancyIdentity {
                tool: ReasonText(identity.tool),
                version: ReasonText(identity.version),
                observed_executable_sha256: sha(identity.observed_executable_sha256)
                    .map_or(ObservedExecutable::NotObserved, ObservedExecutable::Observed),
                profile: Sha256Text(identity.profile_sha256),
                settings: Sha256Text(identity.settings_sha256),
                source: Sha256Text(identity.source_sha256),
                reference_output: Sha256Text(identity.reference_output_sha256),
                intended_output: Sha256Text(identity.intended_output_sha256),
                file_metadata: sha(identity.file_metadata_sha256),
            },
            approved_executable_sha256: wire
                .approved_executable_sha256
                .into_iter()
                .map(Sha256Text)
                .collect(),
            reference_expected: wire
                .reference_expected
                .map_or(ReferenceExpectation::Underivable, |rows| {
                    ReferenceExpectation::Derived(findings(rows))
                }),
            intended_expected: findings(wire.intended_expected),
            reason: ReasonText(wire.reason),
            process: wire.process,
        }
    }
}

/// One case a provider's authoring registers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CaseSchema {
    rule: String,
    name: String,
    case_sha256: String,
    source_sha256: String,
    files: Vec<String>,
    comparison: ComparisonSchema,
    expected: Vec<FindingSchema>,
    project_context_mode: ContextModeSchema,
    context_identity: Option<ContextIdentitySchema>,
    reference_discrepancy: Option<DiscrepancySchema>,
    expectation_contract: Option<ContractSchema>,
    rule_context: Option<serde_json::Value>,
    rule_observations: Option<serde_json::Value>,
    selected_rules: Option<Vec<String>>,
    file_observation: Option<FileObservationSchema>,
    file_metadata: Option<BTreeMap<String, bool>>,
    file_metadata_sha256: Option<String>,
    project_search_roots: Option<Vec<String>>,
    given_project_inventory: Option<InventorySchema>,
    project_inputs_sha256: Option<String>,
}

impl From<CaseSchema> for CaseDocument {
    fn from(wire: CaseSchema) -> Self {
        Self {
            rule: RuleName(wire.rule),
            name: CaseName(wire.name),
            case_sha256: Sha256Text(wire.case_sha256),
            source_sha256: Sha256Text(wire.source_sha256),
            files: wire.files.into_iter().map(path).collect(),
            comparison: match wire.comparison {
                ComparisonSchema::Full => CaseComparison::Full,
                ComparisonSchema::Rule => CaseComparison::Rule,
            },
            expected: findings(wire.expected),
            project_context_mode: wire.project_context_mode.into(),
            context_identity: wire.context_identity.map_or(
                CaseContextIdentity::Isolated,
                |identity| {
                    CaseContextIdentity::Listed(ContextIdentity {
                        mode: identity.mode.into(),
                        logical_root: path(identity.logical_root),
                        manifest_hash: Sha256Text(identity.manifest_hash),
                        present_paths: identity.present_paths.into_iter().map(path).collect(),
                        missing_paths: identity.missing_paths.into_iter().map(path).collect(),
                    })
                },
            ),
            reference_discrepancy: wire
                .reference_discrepancy
                .map_or(CaseReference::Agrees, |wire| CaseReference::Differs(wire.into())),
            expectation_contract: wire
                .expectation_contract
                .map_or(ExpectationContract::Unwritten, |wire| {
                    ExpectationContract::Declared(wire.into())
                }),
            rule_context: canonical(wire.rule_context)
                .map_or(RuleContextRecord::Undeclared, RuleContextRecord::Declared),
            rule_observations: canonical(wire.rule_observations).map_or(
                RuleObservationsRecord::Undeclared,
                RuleObservationsRecord::Declared,
            ),
            selected_rules: wire.selected_rules.map_or(RuleCohort::Undeclared, |rules| {
                RuleCohort::Declared(rules.into_iter().map(RuleName).collect())
            }),
            file_observation: wire
                .file_observation
                .map_or(FileObservation::Undeclared, |value| match value {
                    FileObservationSchema::PosixExecutable => FileObservation::PosixExecutable,
                }),
            file_metadata: wire.file_metadata.map_or(FileMetadata::Undeclared, |values| {
                FileMetadata::Declared(
                    values
                    .into_iter()
                    .map(|(file, executable)| {
                        (
                            path(file),
                            if executable {
                                FileExecutable::Set
                            } else {
                                FileExecutable::Unset
                            },
                        )
                    })
                    .collect(),
                )
            }),
            file_metadata_sha256: FileMetadataDigest::from(sha(wire.file_metadata_sha256)),
            project_search_roots: wire
                .project_search_roots
                .map_or(DeclaredScope::Undeclared, |roots| {
                    DeclaredScope::Roots(roots.into_iter().map(path).collect())
                }),
            given_project_inventory: wire.given_project_inventory.map_or(ProjectInventory::Undeclared, |inventory| {
                ProjectInventory::Declared(ProjectInventoryDocument {
                    completeness: match inventory.completeness {
                        CompletenessSchema::Partial => InventoryCompleteness::Partial,
                        CompletenessSchema::CompleteWithinDeclaredScope => {
                            InventoryCompleteness::CompleteWithinDeclaredScope
                        }
                    },
                    entries: inventory
                        .entries
                        .into_iter()
                        .map(|entry| ProjectEntry {
                            path: path(entry.path),
                            state: match entry.state {
                                EntryStateSchema::File => EntryState::Regular,
                                EntryStateSchema::Directory => EntryState::Directory,
                                EntryStateSchema::Other => EntryState::Other,
                                EntryStateSchema::Missing => EntryState::Missing,
                                EntryStateSchema::Unknown => EntryState::Unknown,
                            },
                        })
                        .collect(),
                })
            }),
            project_inputs_sha256: ProjectInputsDigest::from(sha(wire.project_inputs_sha256)),
        }
    }
}

/// One raw invocation record kept beside a captured process; a surprise field is refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvocationSchema {
    actual_request_root: String,
    files: Vec<String>,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
    exit: i64,
    signal: Option<i64>,
    observer_no_deadline: bool,
    metadata_error: Option<String>,
    source_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
    executable_sha256: Option<String>,
    settings_sha256: Option<String>,
    normalized_settings: Option<serde_json::Value>,
    profile_sha256: Option<String>,
    tool: Option<String>,
    version: Option<String>,
    state: Option<String>,
}

/// The parts of the normalized settings that raw metadata re-derives.
#[derive(Debug, Deserialize)]
struct SettingsSchema {
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    directory: String,
}

fn argument_texts(values: Vec<String>) -> Vec<ArgumentText> {
    values.into_iter().map(ArgumentText).collect()
}

fn environment_texts(
    values: BTreeMap<String, String>,
) -> BTreeMap<EnvironmentName, EnvironmentValue> {
    values
        .into_iter()
        .map(|(name, value)| (EnvironmentName(name), EnvironmentValue(value)))
        .collect()
}

fn pin(value: Option<String>) -> Pin {
    value.map_or(Pin::Absent, |digest| Pin::Sha256(Sha256Text(digest)))
}

impl TryFrom<InvocationSchema> for InvocationDocument {
    type Error = Error;

    fn try_from(wire: InvocationSchema) -> Result<Self, Error> {
        let signal = wire
            .signal
            .map_or(Signal::Absent, |number| Signal::Number(ExitStatus(number)));
        let files = wire.files.into_iter().map(PathText).collect();
        let arguments = argument_texts(wire.arguments);
        let environment = environment_texts(wire.environment);
        if let Some(error) = wire.metadata_error {
            return Ok(Self::Failed(FailedInvocation {
                actual_request_root: PathText(wire.actual_request_root),
                files,
                arguments,
                environment,
                directory: PathText(wire.directory),
                exit: ExitStatus(wire.exit),
                signal,
                observer_no_deadline: YesNo::from(wire.observer_no_deadline),
                metadata_error: ReasonText(error),
                source_sha256: pin(wire.source_sha256),
                file_metadata_sha256: pin(wire.file_metadata_sha256),
            }));
        }
        let (
            Some(executable),
            Some(settings),
            Some(normalized),
            Some(profile),
            Some(tool),
            Some(version),
            Some(state),
        ) = (
            wire.executable_sha256,
            wire.settings_sha256,
            wire.normalized_settings,
            wire.profile_sha256,
            wire.tool,
            wire.version,
            wire.state,
        )
        else {
            return Err(Error::Invalid(constants::INVOCATION_LACKS_PROCESS_IDENTITY.into()));
        };
        let parts: SettingsSchema = serde_json::from_value(normalized.clone())
            .map_err(|error| Error::Json(Box::new(error)))?;
        Ok(Self::Complete(CompleteInvocation {
            actual_request_root: PathText(wire.actual_request_root),
            files,
            executable_sha256: Sha256Text(executable),
            settings_sha256: Sha256Text(settings),
            normalized_settings: NormalizedSettings {
                arguments: argument_texts(parts.arguments),
                environment: environment_texts(parts.environment),
                directory: PathText(parts.directory),
                canonical: CanonicalJson(normalized.to_string()),
            },
            profile_sha256: Sha256Text(profile),
            tool: ReasonText(tool),
            version: ReasonText(version),
            state: PathText(state),
            arguments,
            environment,
            directory: PathText(wire.directory),
            exit: ExitStatus(wire.exit),
            signal,
            observer_no_deadline: YesNo::from(wire.observer_no_deadline),
            source_sha256: pin(wire.source_sha256),
            file_metadata_sha256: pin(wire.file_metadata_sha256),
        }))
    }
}

/// Decode one raw invocation record.
pub(super) fn invocation(bytes: &CapturedBytes) -> Result<InvocationDocument, Error> {
    let wire = serde_json::from_slice::<InvocationSchema>(&bytes.0)
        .map_err(|error| Error::Json(Box::new(error)))?;
    InvocationDocument::try_from(wire)
}

/// Decode the generated registration.
pub(super) fn registration(bytes: &CapturedBytes) -> Result<Registration, Error> {
    serde_json::from_slice::<RegistrationSchema>(&bytes.0)
        .map(Registration::from)
        .map_err(|error| Error::Json(Box::new(error)))
}

/// The one-line JSON answer of an offline receipt audit, newline included.
pub(super) fn audit(audit: &ReceiptAudit) -> Result<CapturedBytes, Error> {
    use ryo_benchmark_contracts::domain::receipts::types::CaseKey;
    let case = |key: &CaseKey| serde_json::json!({"rule": key.rule.0, "name": key.name.0});
    let mut result = serde_json::json!({
        "schema": "benchmark-case-receipt-audit/v1",
        "case_count": audit.case_count.0,
        "validated_case_ids": audit.verified.iter().map(case).collect::<Vec<_>>(),
        "raw_exact": audit.raw_exact.0,
        "documented_discrepancies": audit.documented_discrepancies.0,
        "contract_only": audit.contract_only.0,
        "errors": audit.errors.iter().map(|error| error.0.as_str()).collect::<Vec<_>>(),
    });
    if !audit.metadata_verified.is_empty() {
        result["file_metadata_verified_cases"] = serde_json::json!(
            audit
                .metadata_verified
                .iter()
                .map(|verified| serde_json::json!({"rule": verified.case.rule.0, "name": verified.case.name.0, "sha256": verified.digest.0}))
                .collect::<Vec<_>>()
        );
    }
    if !audit.tool_contract_differences.is_empty() {
        result["tool_contract_difference_case_ids"] = serde_json::json!(
            audit
                .tool_contract_differences
                .iter()
                .map(case)
                .collect::<Vec<_>>()
        );
    }
    let mut bytes =
        serde_json::to_vec(&result).map_err(|error| Error::Json(Box::new(error)))?;
    bytes.push(b'\n');
    Ok(CapturedBytes(bytes))
}

/// One result row as the public summary and the per-rule result file write it.
#[derive(Serialize)]
struct RowSchema {
    rule_id: String,
    status: &'static str,
    reason: Option<String>,
    instance: Option<serde_json::Value>,
    case_results_sha256: Option<String>,
    case_count: usize,
    raw_exact: usize,
    documented_discrepancies: usize,
    contract_only: usize,
}

/// The atomic public inventory snapshot.
#[derive(Serialize)]
struct SummarySchema {
    schema: &'static str,
    provider: String,
    run_state: &'static str,
    identity: BTreeMap<String, serde_json::Value>,
    counts: BTreeMap<&'static str, usize>,
    rules: Vec<RowSchema>,
}

/// The JSON value a canonical JSON text spells.
fn value(text: &CanonicalJson) -> Result<serde_json::Value, Error> {
    serde_json::from_str(&text.0).map_err(|error| Error::Json(Box::new(error)))
}

/// The wire shape of a row.
fn row_schema(row: &Row) -> Result<RowSchema, Error> {
    {
        Ok(RowSchema {
            rule_id: row.rule_id.0.clone(),
            status: match row.status {
                RowStatus::Pass => constants::PASS,
                RowStatus::Fail => constants::FAIL,
            },
            reason: match &row.reason {
                RowReason::Failed(reason) => Some(reason.0.clone()),
                RowReason::Passed => None,
            },
            instance: match &row.instance {
                RowInstance::Carried(text) => Some(value(text)?),
                RowInstance::Absent => None,
            },
            case_results_sha256: match &row.case_results_sha256 {
                ReceiptDigest::Recorded(digest) => Some(digest.0.clone()),
                ReceiptDigest::Unrecorded => None,
            },
            case_count: row.case_count.0,
            raw_exact: row.raw_exact.0,
            documented_discrepancies: row.documented_discrepancies.0,
            contract_only: row.contract_only.0,
        })
    }
}

/// Encode one result row, pretty printed.
pub(super) fn row(row: &Row) -> Result<CapturedBytes, Error> {
    serde_json::to_vec_pretty(&row_schema(row)?)
        .map(CapturedBytes)
        .map_err(|error| Error::Json(Box::new(error)))
}

/// Encode a whole summary, pretty printed, with its final newline.
pub(super) fn summary(summary: &Summary) -> Result<CapturedBytes, Error> {
    let wire = SummarySchema {
        schema: constants::BENCHMARK_INVENTORY_V1,
        provider: summary.provider.0.clone(),
        run_state: match summary.run_state {
            RunState::Running => constants::RUNNING,
            RunState::Complete => constants::COMPLETE,
            RunState::Failed => constants::FAILED,
        },
        identity: summary
            .identity
            .iter()
            .map(|(key, text)| Ok((key.0.clone(), value(text)?)))
            .collect::<Result<_, Error>>()?,
        counts: BTreeMap::from([
            (constants::PASS, summary.passed.0),
            (constants::FAIL, summary.failed.0),
        ]),
        rules: summary
            .rules
            .iter()
            .map(row_schema)
            .collect::<Result<_, _>>()?,
    };
    let mut bytes = serde_json::to_vec_pretty(&wire).map_err(|error| Error::Json(Box::new(error)))?;
    bytes.extend_from_slice(constants::NEWLINE);
    Ok(CapturedBytes(bytes))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
