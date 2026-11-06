//! The wire shapes of the registration and of the native receipt rows. Unknown fields are refused:
//! the documents are a contract, and the python authoring is its source of truth.
use crate::outbound::inventory::types::{
    Command, ExecutablePin, FilePin, RegisteredRule, Registration, RegistrationAuthority,
    case_document::{
        CaseContextIdentity, CaseDocument, Comparison, ContextIdentity,
        EntryState, ExpectationContract, FileMetadata, FileObservation, InventoryCompleteness,
        ProjectEntry, ProjectInventory, ProjectInventoryDocument, ProjectScope,
    },
};
use crate::outbound::suite::types::{
    finding::{ByteOffset, Finding},
    observation_document::{
        ContextMode, FileMetadataDigest, ProjectInputsDigest, ReferenceExpectation, RuleCohort,
        RuleContextRecord, RuleObservationsRecord,
    },
    process::ProcessFailureWire,
    reference_discrepancy::{
        DiscrepancyIdentity, ObservedExecutable, ReferenceDiscrepancy,
    },
    text::{CanonicalJson, CaseName, PathText, ReasonText, RuleName, Sha256Text},
};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
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
                ComparisonSchema::Full => Comparison::Full,
                ComparisonSchema::Rule => Comparison::Rule,
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
            reference_discrepancy: wire.reference_discrepancy.map(Into::into),
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
            file_metadata_sha256: sha(wire.file_metadata_sha256),
            project_search_roots: wire
                .project_search_roots
                .map_or(ProjectScope::Undeclared, |roots| {
                    ProjectScope::Roots(roots.into_iter().map(path).collect())
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
                                EntryStateSchema::File => EntryState::File,
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
