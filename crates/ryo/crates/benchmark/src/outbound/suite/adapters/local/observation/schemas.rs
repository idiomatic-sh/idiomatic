//! The native receipt row: read back by the inventory and written by the case observer.
//! Unknown fields are refused; the documents are a contract.
use ryo_benchmark_contracts::domain::documents::types::{finding::{FindingOffset, Finding, SkippedPair}, observation_document::{
        ContextManifest, ContextMode, ContextRun, ExitStatus, FileMetadataDigest, IntentMatch,
        ObservationDocument, ProcessArtifacts, ProcessObservation, ProjectContextRecord,
        ProjectInputsDigest, ReferenceExpectation, RuleCohort, RuleContextRecord,
        RuleObservationsRecord, SourceIdentity, ToolContractRecord,
    }, reference_discrepancy::{ArtifactPointer, DiscrepancyValidation, ExpectationKind}, text::{
        CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, ContestantSide,
        YesNo,
    }};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::{Deserialize, Serialize};

fn sha(value: Option<String>) -> Option<Sha256Text> {
    value.map(Sha256Text)
}
fn text(value: &Sha256Text) -> String {
    value.0.clone()
}
fn opt_text(value: &Option<Sha256Text>) -> Option<String> {
    value.as_ref().map(text)
}
fn canonical(value: Option<serde_json::Value>) -> Option<CanonicalJson> {
    value.map(|value| CanonicalJson(value.to_string()))
}
fn flag(value: bool) -> YesNo {
    YesNo::from(value)
}
fn is(value: YesNo) -> bool {
    value == YesNo::Yes
}
fn sealed(value: &SourceIdentity) -> Option<String> {
    match value {
        SourceIdentity::Sealed(digest) => Some(text(digest)),
        SourceIdentity::Unavailable => None,
    }
}
fn metadata_digest(value: &FileMetadataDigest) -> Option<String> {
    match value {
        FileMetadataDigest::Declared(digest) => Some(text(digest)),
        FileMetadataDigest::Undeclared => None,
    }
}
fn inputs_digest(value: &ProjectInputsDigest) -> Option<String> {
    match value {
        ProjectInputsDigest::Declared(digest) => Some(text(digest)),
        ProjectInputsDigest::Undeclared => None,
    }
}
fn json_tree(value: &CanonicalJson) -> serde_json::Value {
    serde_json::from_str(&value.0).unwrap_or(serde_json::Value::Null)
}

#[derive(Debug, Deserialize, Serialize)]
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
impl From<&Finding> for FindingSchema {
    fn from(value: &Finding) -> Self {
        Self {
            rule: value.rule.0.clone(),
            file: value.file.0.clone(),
            start: value.start.0,
            end: value.end.0,
        }
    }
}
fn findings(rows: Vec<FindingSchema>) -> Vec<Finding> {
    rows.into_iter().map(Finding::from).collect()
}
fn wire_findings(rows: &[Finding]) -> Vec<FindingSchema> {
    rows.iter().map(FindingSchema::from).collect()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SideSchema {
    Reference,
    Candidate,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SkippedSchema {
    rule: String,
    path: String,
    reason: SkipReasonSchema,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
enum ExecutionSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CoverageSchema {
    selected_rule_file_pairs: usize,
    evaluated_rule_file_pairs: usize,
    skipped_rule_file_pairs: usize,
    unknown_or_unread_rule_file_pairs: usize,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContextManifestSchema {
    mode: ModeSchema,
    logical_root: String,
    actual_root: String,
    manifest_hash: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ContractSchema {
    Diagnostics,
    CompleteReport,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum KindSchema {
    Diagnostics,
    ProcessFailure,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum BasisSchema {
    LanguageDefect,
    ToolContractDifference,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PointerSchema {
    path: String,
    sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessArtifactsSchema {
    stdout: PointerSchema,
    stderr: PointerSchema,
    invocation: PointerSchema,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessObservationSchema {
    kind: KindSchema,
    exit: i64,
    signal: Option<i64>,
    actual_request_root: String,
    observer_no_deadline: bool,
    executable_sha256: Option<String>,
    settings_sha256: Option<String>,
    metadata_error: Option<String>,
    template_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifacts: Option<ProcessArtifactsSchema>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ValidationSchema {
    discrepancy_basis: BasisSchema,
    process_observation_group_sha256: Option<String>,
    reference_expectation_kind: KindSchema,
    settings_sha256: String,
    actual_executable_sha256: String,
    profile_sha256: String,
    source_sha256: String,
    reference_output_sha256: String,
    intended_output_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_metadata_sha256: Option<String>,
}

/// One native side's receipt row.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ObservationSchema {
    rule: String,
    name: String,
    side: SideSchema,
    complete: bool,
    error: Option<String>,
    case_passed: bool,
    files: Vec<String>,
    source_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_metadata_sha256: Option<String>,
    project_inputs_sha256: Option<String>,
    expected: Vec<FindingSchema>,
    reference_expected: Option<Vec<FindingSchema>>,
    diagnostics: Option<Vec<FindingSchema>>,
    skipped: Option<Vec<SkippedSchema>>,
    observation_schema: String,
    expectation_contract: ContractSchema,
    execution_metadata: Option<ExecutionSchema>,
    coverage: Option<CoverageSchema>,
    context_manifest: Option<ContextManifestSchema>,
    project_context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_observations: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_rules: Option<Vec<String>>,
    actual_request_root: Option<String>,
    documented_reference_discrepancy: bool,
    documented_tool_contract_difference: Option<bool>,
    discrepancy_validation: Option<ValidationSchema>,
    raw_parity_matches_intended: Option<bool>,
    diagnostics_match_intended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_observation: Option<ProcessObservationSchema>,
}

fn contract(wire: ContractSchema) -> ComparisonContract {
    match wire {
        ContractSchema::Diagnostics => ComparisonContract::Diagnostics,
        ContractSchema::CompleteReport => ComparisonContract::CompleteReport,
    }
}
fn wire_contract(value: ComparisonContract) -> ContractSchema {
    match value {
        ComparisonContract::Diagnostics => ContractSchema::Diagnostics,
        ComparisonContract::CompleteReport => ContractSchema::CompleteReport,
    }
}
fn kind(wire: KindSchema) -> ExpectationKind {
    match wire {
        KindSchema::Diagnostics => ExpectationKind::Diagnostics,
        KindSchema::ProcessFailure => ExpectationKind::ProcessFailure,
    }
}
fn wire_kind(value: ExpectationKind) -> KindSchema {
    match value {
        ExpectationKind::Diagnostics => KindSchema::Diagnostics,
        ExpectationKind::ProcessFailure => KindSchema::ProcessFailure,
    }
}
fn basis(wire: BasisSchema) -> DiscrepancyBasis {
    match wire {
        BasisSchema::LanguageDefect => DiscrepancyBasis::LanguageDefect,
        BasisSchema::ToolContractDifference => DiscrepancyBasis::ToolContract,
    }
}
fn wire_basis(value: DiscrepancyBasis) -> BasisSchema {
    match value {
        DiscrepancyBasis::LanguageDefect => BasisSchema::LanguageDefect,
        DiscrepancyBasis::ToolContract => BasisSchema::ToolContractDifference,
    }
}
fn pointer(wire: PointerSchema) -> ArtifactPointer {
    ArtifactPointer {
        path: PathText(wire.path),
        sha256: Sha256Text(wire.sha256),
    }
}
fn wire_pointer(value: &ArtifactPointer) -> PointerSchema {
    PointerSchema {
        path: value.path.0.clone(),
        sha256: value.sha256.0.clone(),
    }
}

impl From<ObservationSchema> for ObservationDocument {
    fn from(wire: ObservationSchema) -> Self {
        Self {
            rule: RuleName(wire.rule),
            name: CaseName(wire.name),
            side: match wire.side {
                SideSchema::Reference => ContestantSide::Reference,
                SideSchema::Candidate => ContestantSide::Candidate,
            },
            complete: flag(wire.complete),
            error: wire.error.map(ReasonText),
            case_passed: flag(wire.case_passed),
            files: wire.files.into_iter().map(PathText).collect(),
            source_sha256: sha(wire.source_sha256)
                .map_or(SourceIdentity::Unavailable, SourceIdentity::Sealed),
            file_metadata_sha256: FileMetadataDigest::from(sha(wire.file_metadata_sha256)),
            project_inputs_sha256: ProjectInputsDigest::from(sha(wire.project_inputs_sha256)),
            expected: findings(wire.expected),
            reference_expected: wire
                .reference_expected
                .map_or(ReferenceExpectation::Underivable, |rows| {
                    ReferenceExpectation::Derived(findings(rows))
                }),
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
            observation_schema: SchemaName(wire.observation_schema),
            expectation_contract: contract(wire.expectation_contract),
            execution_metadata: wire.execution_metadata.map(|value| match value {
                ExecutionSchema::Unknown {} => ExecutionMetadata::Unknown,
                ExecutionSchema::Unavailable { reason } => {
                    ExecutionMetadata::Unavailable { reason }
                }
                ExecutionSchema::Complete {} => ExecutionMetadata::Complete,
            }),
            coverage: wire.coverage.map(|coverage| RuleCoverage {
                selected: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.selected_rule_file_pairs),
                evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.evaluated_rule_file_pairs),
                skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.skipped_rule_file_pairs),
                unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.unknown_or_unread_rule_file_pairs),
            }),
            context_manifest: wire.context_manifest.map_or(ContextRun::Isolated, |manifest| {
                ContextRun::Listed(ContextManifest {
                    mode: match manifest.mode {
                        ModeSchema::Isolated => ContextMode::Isolated,
                        ModeSchema::ListedDisk => ContextMode::ListedDisk,
                    },
                    logical_root: PathText(manifest.logical_root),
                    actual_root: PathText(manifest.actual_root),
                    manifest_hash: Sha256Text(manifest.manifest_hash),
                })
            }),
            project_context: canonical(wire.project_context)
                .map_or(ProjectContextRecord::NotCarried, ProjectContextRecord::Carried),
            rule_context: canonical(wire.rule_context)
                .map_or(RuleContextRecord::Undeclared, RuleContextRecord::Declared),
            rule_observations: canonical(wire.rule_observations).map_or(
                RuleObservationsRecord::Undeclared,
                RuleObservationsRecord::Declared,
            ),
            selected_rules: wire.selected_rules.map_or(RuleCohort::Undeclared, |rules| {
                RuleCohort::Declared(rules.into_iter().map(RuleName).collect())
            }),
            actual_request_root: wire.actual_request_root.map(PathText),
            documented_reference_discrepancy: flag(wire.documented_reference_discrepancy),
            documented_tool_contract_difference: wire
                .documented_tool_contract_difference
                .map_or(ToolContractRecord::Unrecorded, |value| {
                    ToolContractRecord::Recorded(flag(value))
                }),
            discrepancy_validation: wire.discrepancy_validation.map(|proof| {
                DiscrepancyValidation {
                    discrepancy_basis: basis(proof.discrepancy_basis),
                    process_observation_group_sha256: sha(
                        proof.process_observation_group_sha256
                    ),
                    reference_expectation_kind: kind(proof.reference_expectation_kind),
                    settings_sha256: Sha256Text(proof.settings_sha256),
                    actual_executable_sha256: Sha256Text(proof.actual_executable_sha256),
                    profile_sha256: Sha256Text(proof.profile_sha256),
                    source_sha256: Sha256Text(proof.source_sha256),
                    reference_output_sha256: Sha256Text(proof.reference_output_sha256),
                    intended_output_sha256: Sha256Text(proof.intended_output_sha256),
                    file_metadata_sha256: sha(proof.file_metadata_sha256),
                }
            }),
            raw_parity_matches_intended: wire.raw_parity_matches_intended.map(flag),
            diagnostics_match_intended: wire
                .diagnostics_match_intended
                .map_or(IntentMatch::Unreported, |value| IntentMatch::Matched(flag(value))),
            process_observation: wire.process_observation.map(|process| ProcessObservation {
                kind: kind(process.kind),
                exit: ExitStatus(process.exit),
                signal: process.signal.map(ExitStatus),
                actual_request_root: PathText(process.actual_request_root),
                observer_no_deadline: flag(process.observer_no_deadline),
                executable_sha256: sha(process.executable_sha256),
                settings_sha256: sha(process.settings_sha256),
                metadata_error: process.metadata_error.map(ReasonText),
                template_sha256: sha(process.template_sha256),
                artifacts: process.artifacts.map(|artifacts| ProcessArtifacts {
                    stdout: pointer(artifacts.stdout),
                    stderr: pointer(artifacts.stderr),
                    invocation: pointer(artifacts.invocation),
                }),
            }),
        }
    }
}

impl From<&ObservationDocument> for ObservationSchema {
    fn from(value: &ObservationDocument) -> Self {
        Self {
            rule: value.rule.0.clone(),
            name: value.name.0.clone(),
            side: match value.side {
                ContestantSide::Reference => SideSchema::Reference,
                ContestantSide::Candidate => SideSchema::Candidate,
            },
            complete: is(value.complete),
            error: value.error.as_ref().map(|error| error.0.clone()),
            case_passed: is(value.case_passed),
            files: value.files.iter().map(|file| file.0.clone()).collect(),
            source_sha256: sealed(&value.source_sha256),
            file_metadata_sha256: metadata_digest(&value.file_metadata_sha256),
            project_inputs_sha256: inputs_digest(&value.project_inputs_sha256),
            expected: wire_findings(&value.expected),
            reference_expected: match &value.reference_expected {
                ReferenceExpectation::Derived(rows) => Some(wire_findings(rows)),
                ReferenceExpectation::Underivable => None,
            },
            diagnostics: value.diagnostics.as_deref().map(wire_findings),
            skipped: value.skipped.as_ref().map(|rows| {
                rows.iter()
                    .map(|skip| SkippedSchema {
                        rule: skip.rule.0.clone(),
                        path: skip.path.0.clone(),
                        reason: SkipReasonSchema::SyntaxRejected,
                    })
                    .collect()
            }),
            observation_schema: value.observation_schema.0.clone(),
            expectation_contract: wire_contract(value.expectation_contract),
            execution_metadata: value.execution_metadata.as_ref().map(
                |metadata| match metadata {
                    ExecutionMetadata::Unknown => ExecutionSchema::Unknown {},
                    ExecutionMetadata::Unavailable { reason } => ExecutionSchema::Unavailable {
                        reason: reason.clone(),
                    },
                    ExecutionMetadata::Complete => ExecutionSchema::Complete {},
                },
            ),
            coverage: value.coverage.map(|coverage| CoverageSchema {
                selected_rule_file_pairs: coverage.selected,
                evaluated_rule_file_pairs: coverage.evaluated,
                skipped_rule_file_pairs: coverage.skipped,
                unknown_or_unread_rule_file_pairs: coverage.unknown_or_unread,
            }),
            context_manifest: match &value.context_manifest {
                ContextRun::Isolated => None,
                ContextRun::Listed(manifest) => Some(ContextManifestSchema {
                    mode: match manifest.mode {
                        ContextMode::Isolated => ModeSchema::Isolated,
                        ContextMode::ListedDisk => ModeSchema::ListedDisk,
                    },
                    logical_root: manifest.logical_root.0.clone(),
                    actual_root: manifest.actual_root.0.clone(),
                    manifest_hash: manifest.manifest_hash.0.clone(),
                }),
            },
            project_context: match &value.project_context {
                ProjectContextRecord::Carried(json) => Some(json_tree(json)),
                ProjectContextRecord::NotCarried => None,
            },
            rule_context: match &value.rule_context {
                RuleContextRecord::Declared(json) => Some(json_tree(json)),
                RuleContextRecord::Undeclared => None,
            },
            rule_observations: match &value.rule_observations {
                RuleObservationsRecord::Declared(json) => Some(json_tree(json)),
                RuleObservationsRecord::Undeclared => None,
            },
            selected_rules: match &value.selected_rules {
                RuleCohort::Declared(rules) => {
                    Some(rules.iter().map(|rule| rule.0.clone()).collect())
                }
                RuleCohort::Undeclared => None,
            },
            actual_request_root: value
                .actual_request_root
                .as_ref()
                .map(|root| root.0.clone()),
            documented_reference_discrepancy: is(value.documented_reference_discrepancy),
            documented_tool_contract_difference: match value.documented_tool_contract_difference {
                ToolContractRecord::Recorded(answer) => Some(is(answer)),
                ToolContractRecord::Unrecorded => None,
            },
            discrepancy_validation: value.discrepancy_validation.as_ref().map(|proof| {
                ValidationSchema {
                    discrepancy_basis: wire_basis(proof.discrepancy_basis),
                    process_observation_group_sha256: opt_text(
                        &proof.process_observation_group_sha256,
                    ),
                    reference_expectation_kind: wire_kind(proof.reference_expectation_kind),
                    settings_sha256: text(&proof.settings_sha256),
                    actual_executable_sha256: text(&proof.actual_executable_sha256),
                    profile_sha256: text(&proof.profile_sha256),
                    source_sha256: text(&proof.source_sha256),
                    reference_output_sha256: text(&proof.reference_output_sha256),
                    intended_output_sha256: text(&proof.intended_output_sha256),
                    file_metadata_sha256: opt_text(&proof.file_metadata_sha256),
                }
            }),
            raw_parity_matches_intended: value.raw_parity_matches_intended.map(is),
            diagnostics_match_intended: match value.diagnostics_match_intended {
                IntentMatch::Matched(answer) => Some(is(answer)),
                IntentMatch::Unreported => None,
            },
            process_observation: value.process_observation.as_ref().map(|process| {
                ProcessObservationSchema {
                    kind: wire_kind(process.kind),
                    exit: process.exit.0,
                    signal: process.signal.map(|signal| signal.0),
                    actual_request_root: process.actual_request_root.0.clone(),
                    observer_no_deadline: is(process.observer_no_deadline),
                    executable_sha256: opt_text(&process.executable_sha256),
                    settings_sha256: opt_text(&process.settings_sha256),
                    metadata_error: process.metadata_error.as_ref().map(|e| e.0.clone()),
                    template_sha256: opt_text(&process.template_sha256),
                    artifacts: process.artifacts.as_ref().map(|artifacts| {
                        ProcessArtifactsSchema {
                            stdout: wire_pointer(&artifacts.stdout),
                            stderr: wire_pointer(&artifacts.stderr),
                            invocation: wire_pointer(&artifacts.invocation),
                        }
                    }),
                }
            }),
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
