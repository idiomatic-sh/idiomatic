//! The wire shapes of the registration and of the native receipt rows. Unknown fields are refused:
//! the documents are a contract, and the python authoring is its source of truth.
use crate::outbound::inventory::types::{
    Command, ExecutablePin, FilePin, RegisteredRule, Registration, RegistrationAuthority,
    case_document::{
        CaseDocument, Comparison, ContextIdentity, ContextMode, EntryState, FileObservation,
        InventoryCompleteness, ProjectEntry, ProjectInventoryDocument,
    },
    discrepancy::{
        ArtifactPointer, DiscrepancyIdentity, DiscrepancyValidation, ExpectationKind,
        ReferenceDiscrepancy,
    },
    finding::{ByteOffset, Finding, SkippedPair},
    observation_document::{
        ContextManifest, ExitStatus, ObservationDocument, ProcessArtifacts, ProcessObservation,
    },
    text::{
        CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, Side,
        YesNo,
    },
};
use crate::outbound::suite::types::process::ProcessFailureWire;
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::Deserialize;
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
fn flag(value: Option<bool>) -> Option<YesNo> {
    value.map(YesNo::from)
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
                AuthoritySchema::Unavailable { reason } => {
                    RegistrationAuthority::Unavailable { reason }
                }
                AuthoritySchema::Execution {
                    profile,
                    profile_sha256,
                    reference,
                    worker,
                } => RegistrationAuthority::Execution {
                    profile,
                    profile_sha256,
                    reference: ExecutablePin {
                        path: reference.path,
                        sha256: reference.sha256,
                    },
                    worker: ExecutablePin {
                        path: worker.path,
                        sha256: worker.sha256,
                    },
                },
            },
            provider: wire.provider,
            defaults: wire.defaults,
            rules: wire.rules.into_iter().map(RegisteredRule::from).collect(),
            identity: wire.identity,
        }
    }
}

impl From<RegisteredRuleSchema> for RegisteredRule {
    fn from(wire: RegisteredRuleSchema) -> Self {
        let pin = |pin: FilePinSchema| FilePin {
            path: pin.path,
            sha256: pin.sha256,
        };
        Self {
            definition: wire.definition.map(pin),
            modules: wire
                .modules
                .into_iter()
                .map(|(name, value)| (name, pin(value)))
                .collect(),
            artifacts: wire.artifacts,
            rule_id: wire.rule_id,
            command: wire.command.map(|command| match command {
                CommandSchema::CaseVerification { context } => {
                    Command::CaseVerification { context }
                }
            }),
            reason: wire.reason,
            cases: wire.cases.into_iter().map(CaseDocument::from).collect(),
            instance: wire.instance,
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
            start: ByteOffset(wire.start),
            end: ByteOffset(wire.end),
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

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct IdentitySchema {
    tool: Option<String>,
    version: Option<String>,
    observed_executable_sha256: Option<String>,
    profile_sha256: Option<String>,
    settings_sha256: Option<String>,
    source_sha256: Option<String>,
    reference_output_sha256: Option<String>,
    intended_output_sha256: Option<String>,
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
    reason: Option<String>,
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
                tool: identity.tool.map(ReasonText),
                version: identity.version.map(ReasonText),
                observed_executable_sha256: sha(identity.observed_executable_sha256),
                profile: sha(identity.profile_sha256),
                settings: sha(identity.settings_sha256),
                source: sha(identity.source_sha256),
                reference_output: sha(identity.reference_output_sha256),
                intended_output: sha(identity.intended_output_sha256),
                file_metadata: sha(identity.file_metadata_sha256),
            },
            approved_executable_sha256: wire
                .approved_executable_sha256
                .into_iter()
                .map(Sha256Text)
                .collect(),
            reference_expected: wire.reference_expected.map(findings),
            intended_expected: findings(wire.intended_expected),
            reason: wire.reason.map(ReasonText),
            process: wire.process,
        }
    }
}

/// One case a provider's authoring registers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CaseSchema {
    rule: Option<String>,
    name: Option<String>,
    case_sha256: Option<String>,
    source_sha256: Option<String>,
    files: Vec<String>,
    comparison: Option<ComparisonSchema>,
    expected: Vec<FindingSchema>,
    project_context_mode: Option<ContextModeSchema>,
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
            rule: wire.rule.map(RuleName),
            name: wire.name.map(CaseName),
            case_sha256: sha(wire.case_sha256),
            source_sha256: sha(wire.source_sha256),
            files: wire.files.into_iter().map(path).collect(),
            comparison: match wire.comparison {
                Some(ComparisonSchema::Full) => Comparison::Full,
                Some(ComparisonSchema::Rule) | None => Comparison::Rule,
            },
            expected: findings(wire.expected),
            project_context_mode: wire
                .project_context_mode
                .map_or(ContextMode::Isolated, Into::into),
            context_identity: wire.context_identity.map(|identity| ContextIdentity {
                mode: identity.mode.into(),
                logical_root: path(identity.logical_root),
                manifest_hash: Sha256Text(identity.manifest_hash),
                present_paths: identity.present_paths.into_iter().map(path).collect(),
                missing_paths: identity.missing_paths.into_iter().map(path).collect(),
            }),
            reference_discrepancy: wire.reference_discrepancy.map(Into::into),
            expectation_contract: wire.expectation_contract.map(Into::into),
            rule_context: canonical(wire.rule_context),
            rule_observations: canonical(wire.rule_observations),
            selected_rules: wire
                .selected_rules
                .map(|rules| rules.into_iter().map(RuleName).collect()),
            file_observation: wire.file_observation.map(|value| match value {
                FileObservationSchema::PosixExecutable => FileObservation::PosixExecutable,
            }),
            file_metadata: wire.file_metadata.map(|values| {
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
                    .collect()
            }),
            file_metadata_sha256: sha(wire.file_metadata_sha256),
            project_search_roots: wire
                .project_search_roots
                .map(|roots| roots.into_iter().map(path).collect()),
            given_project_inventory: wire.given_project_inventory.map(|inventory| {
                ProjectInventoryDocument {
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
                                EntryStateSchema::File => EntryState::File,
                                EntryStateSchema::Directory => EntryState::Directory,
                                EntryStateSchema::Other => EntryState::Other,
                                EntryStateSchema::Missing => EntryState::Missing,
                                EntryStateSchema::Unknown => EntryState::Unknown,
                            },
                        })
                        .collect(),
                }
            }),
            project_inputs_sha256: sha(wire.project_inputs_sha256),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SideSchema {
    Reference,
    Candidate,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkippedSchema {
    rule: String,
    path: String,
    reason: SkipReasonSchema,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
enum ExecutionSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageSchema {
    selected_rule_file_pairs: usize,
    evaluated_rule_file_pairs: usize,
    skipped_rule_file_pairs: usize,
    unknown_or_unread_rule_file_pairs: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextManifestSchema {
    mode: Option<ContextModeSchema>,
    logical_root: Option<String>,
    actual_root: Option<String>,
    manifest_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ExpectationKindSchema {
    Diagnostics,
    ProcessFailure,
}

impl From<ExpectationKindSchema> for ExpectationKind {
    fn from(wire: ExpectationKindSchema) -> Self {
        match wire {
            ExpectationKindSchema::Diagnostics => Self::Diagnostics,
            ExpectationKindSchema::ProcessFailure => Self::ProcessFailure,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactPointerSchema {
    path: String,
    sha256: String,
}

impl From<ArtifactPointerSchema> for ArtifactPointer {
    fn from(wire: ArtifactPointerSchema) -> Self {
        Self {
            path: PathText(wire.path),
            sha256: Sha256Text(wire.sha256),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessArtifactsSchema {
    stdout: ArtifactPointerSchema,
    stderr: ArtifactPointerSchema,
    invocation: ArtifactPointerSchema,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessObservationSchema {
    kind: Option<ExpectationKindSchema>,
    exit: Option<i64>,
    signal: Option<i64>,
    actual_request_root: Option<String>,
    observer_no_deadline: Option<bool>,
    executable_sha256: Option<String>,
    settings_sha256: Option<String>,
    metadata_error: Option<String>,
    template_sha256: Option<String>,
    artifacts: Option<ProcessArtifactsSchema>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationSchema {
    discrepancy_basis: Option<BasisSchema>,
    process_observation_group_sha256: Option<String>,
    reference_expectation_kind: Option<ExpectationKindSchema>,
    settings_sha256: Option<String>,
    actual_executable_sha256: Option<String>,
    profile_sha256: Option<String>,
    source_sha256: Option<String>,
    reference_output_sha256: Option<String>,
    intended_output_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
}

/// One native side's receipt row.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObservationSchema {
    rule: Option<String>,
    name: Option<String>,
    side: Option<SideSchema>,
    complete: Option<bool>,
    error: Option<String>,
    case_passed: Option<bool>,
    files: Option<Vec<String>>,
    source_sha256: Option<String>,
    file_metadata_sha256: Option<String>,
    project_inputs_sha256: Option<String>,
    expected: Option<Vec<FindingSchema>>,
    reference_expected: Option<Vec<FindingSchema>>,
    diagnostics: Option<Vec<FindingSchema>>,
    skipped: Option<Vec<SkippedSchema>>,
    observation_schema: Option<String>,
    expectation_contract: Option<ContractSchema>,
    execution_metadata: Option<ExecutionSchema>,
    coverage: Option<CoverageSchema>,
    context_manifest: Option<ContextManifestSchema>,
    project_context: Option<serde_json::Value>,
    rule_context: Option<serde_json::Value>,
    rule_observations: Option<serde_json::Value>,
    selected_rules: Option<Vec<String>>,
    actual_request_root: Option<String>,
    documented_reference_discrepancy: Option<bool>,
    documented_tool_contract_difference: Option<bool>,
    discrepancy_validation: Option<ValidationSchema>,
    raw_parity_matches_intended: Option<bool>,
    diagnostics_match_intended: Option<bool>,
    process_observation: Option<ProcessObservationSchema>,
}

impl From<ObservationSchema> for ObservationDocument {
    fn from(wire: ObservationSchema) -> Self {
        Self {
            rule: wire.rule.map(RuleName),
            name: wire.name.map(CaseName),
            side: wire.side.map(|side| match side {
                SideSchema::Reference => Side::Reference,
                SideSchema::Candidate => Side::Candidate,
            }),
            complete: flag(wire.complete),
            error: wire.error.map(ReasonText),
            case_passed: flag(wire.case_passed),
            files: wire
                .files
                .map(|files| files.into_iter().map(path).collect()),
            source_sha256: sha(wire.source_sha256),
            file_metadata_sha256: sha(wire.file_metadata_sha256),
            project_inputs_sha256: sha(wire.project_inputs_sha256),
            expected: wire.expected.map(findings),
            reference_expected: wire.reference_expected.map(findings),
            diagnostics: wire.diagnostics.map(findings),
            skipped: wire.skipped.map(|rows| {
                rows.into_iter()
                    .map(|skip| {
                        let SkipReasonSchema::SyntaxRejected = skip.reason;
                        SkippedPair {
                            rule: RuleName(skip.rule),
                            path: PathText(skip.path),
                        }
                    })
                    .collect()
            }),
            observation_schema: wire.observation_schema.map(SchemaName),
            expectation_contract: wire.expectation_contract.map(Into::into),
            execution_metadata: wire.execution_metadata.map(|value| match value {
                ExecutionSchema::Unknown {} => ExecutionMetadata::Unknown,
                ExecutionSchema::Unavailable { reason } => {
                    ExecutionMetadata::Unavailable { reason }
                }
                ExecutionSchema::Complete {} => ExecutionMetadata::Complete,
            }),
            coverage: wire.coverage.map(|coverage| RuleCoverage {
                selected: coverage.selected_rule_file_pairs,
                evaluated: coverage.evaluated_rule_file_pairs,
                skipped: coverage.skipped_rule_file_pairs,
                unknown_or_unread: coverage.unknown_or_unread_rule_file_pairs,
            }),
            context_manifest: wire.context_manifest.map(|manifest| ContextManifest {
                mode: manifest.mode.map(Into::into),
                logical_root: manifest.logical_root.map(PathText),
                actual_root: manifest.actual_root.map(PathText),
                manifest_hash: sha(manifest.manifest_hash),
            }),
            project_context: canonical(wire.project_context),
            rule_context: canonical(wire.rule_context),
            rule_observations: canonical(wire.rule_observations),
            selected_rules: wire
                .selected_rules
                .map(|rules| rules.into_iter().map(RuleName).collect()),
            actual_request_root: wire.actual_request_root.map(PathText),
            documented_reference_discrepancy: flag(wire.documented_reference_discrepancy),
            documented_tool_contract_difference: flag(wire.documented_tool_contract_difference),
            discrepancy_validation: wire.discrepancy_validation.map(|proof| {
                DiscrepancyValidation {
                    discrepancy_basis: proof.discrepancy_basis.map(Into::into),
                    process_observation_group_sha256: sha(proof.process_observation_group_sha256),
                    reference_expectation_kind: proof.reference_expectation_kind.map(Into::into),
                    settings_sha256: sha(proof.settings_sha256),
                    actual_executable_sha256: sha(proof.actual_executable_sha256),
                    profile_sha256: sha(proof.profile_sha256),
                    source_sha256: sha(proof.source_sha256),
                    reference_output_sha256: sha(proof.reference_output_sha256),
                    intended_output_sha256: sha(proof.intended_output_sha256),
                    file_metadata_sha256: sha(proof.file_metadata_sha256),
                }
            }),
            raw_parity_matches_intended: flag(wire.raw_parity_matches_intended),
            diagnostics_match_intended: flag(wire.diagnostics_match_intended),
            process_observation: wire.process_observation.map(|process| ProcessObservation {
                kind: process.kind.map(Into::into),
                exit: process.exit.map(ExitStatus),
                signal: process.signal.map(ExitStatus),
                actual_request_root: process.actual_request_root.map(PathText),
                observer_no_deadline: flag(process.observer_no_deadline),
                executable_sha256: sha(process.executable_sha256),
                settings_sha256: sha(process.settings_sha256),
                metadata_error: process.metadata_error.map(ReasonText),
                template_sha256: sha(process.template_sha256),
                artifacts: process.artifacts.map(|artifacts| ProcessArtifacts {
                    stdout: artifacts.stdout.into(),
                    stderr: artifacts.stderr.into(),
                    invocation: artifacts.invocation.into(),
                }),
            }),
        }
    }
}
