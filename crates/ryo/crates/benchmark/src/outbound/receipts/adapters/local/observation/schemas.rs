//! The native receipt row: read back by the inventory and written by the case observer.
//! Unknown fields are refused; the documents are a contract.
use super::super::process_row::schemas::{KindSchema, ProcessRowSchema};
use crate::outbound::receipts::errors::ReceiptsError;
use ryo_benchmark_contracts::domain::documents::types::finding::value::{
    ExpectedFindings, FindingOffset, RecordedFinding, SkippedPair,
};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ContextMode, ContextRun, IntentMatch, ListedContext, ObservationDocument, ProcessArtifacts,
    ProcessRecord, ProjectContextRecord, RuleCohort, RuleContextRecord, RuleObservationsRecord,
    SourceIdentity, ToolContractRecord,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::DiscrepancyValidation;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, CaseName, ContestantSide, FileMetadataDigest, PathText, ProjectInputsDigest,
    ReasonText, RuleName, SchemaName, Sha256Text, YesNo,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, CoverageAccounting, ExecutionMetadata,
};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::Serialize;

fn is(value: YesNo) -> bool {
    value == YesNo::Yes
}
fn text(value: &Sha256Text) -> String {
    value.0.clone()
}
fn pin_text(
    value: &ryo_benchmark_contracts::domain::documents::types::text::value::Pin,
) -> Option<String> {
    match value {
        ryo_benchmark_contracts::domain::documents::types::text::value::Pin::Sha256(digest) => {
            Some(text(digest))
        }
        ryo_benchmark_contracts::domain::documents::types::text::value::Pin::Absent => None,
    }
}
fn reason_text(
    value: &ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason,
) -> Option<String> {
    match value {
        ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason::Stated(reason) => Some(reason.0.clone()),
        ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason::Absent => None,
    }
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

#[derive(Debug, Serialize)]
struct FindingSchema {
    rule: String,
    file: String,
    start: u64,
    end: u64,
}
impl From<&RecordedFinding> for FindingSchema {
    fn from(value: &RecordedFinding) -> Self {
        Self {
            rule: value.rule.0.clone(),
            file: value.file.0.clone(),
            start: value.span.start.0,
            end: value.span.end.0,
        }
    }
}
fn wire_findings(rows: &[RecordedFinding]) -> Vec<FindingSchema> {
    rows.iter().map(FindingSchema::from).collect()
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SideSchema {
    Reference,
    Candidate,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}
#[derive(Debug, Serialize)]
struct SkippedSchema {
    rule: String,
    path: String,
    reason: SkipReasonSchema,
}
#[derive(Debug, Serialize)]
#[serde(tag = "availability", rename_all = "kebab-case")]
enum ExecutionSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}
#[derive(Debug, Serialize)]
struct CoverageSchema {
    selected_rule_file_pairs: usize,
    evaluated_rule_file_pairs: usize,
    skipped_rule_file_pairs: usize,
    unknown_or_unread_rule_file_pairs: usize,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}
#[derive(Debug, Serialize)]
struct ContextManifestSchema {
    mode: ModeSchema,
    logical_root: String,
    actual_root: String,
    manifest_hash: String,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ContractSchema {
    Diagnostics,
    CompleteReport,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum BasisSchema {
    LanguageDefect,
    ToolContractDifference,
}
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
struct ObservationSchema {
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
    process_observation: Option<ProcessRowSchema>,
}

fn wire_contract(value: ComparisonContract) -> ContractSchema {
    match value {
        ComparisonContract::Diagnostics => ContractSchema::Diagnostics,
        ComparisonContract::CompleteReport => ContractSchema::CompleteReport,
    }
}
fn wire_basis(value: DiscrepancyBasis) -> BasisSchema {
    match value {
        DiscrepancyBasis::LanguageDefect => BasisSchema::LanguageDefect,
        DiscrepancyBasis::ToolContract => BasisSchema::ToolContractDifference,
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
            error: reason_text(&value.error),
            case_passed: is(value.case_passed),
            files: value.files.iter().map(|file| file.0.clone()).collect(),
            source_sha256: sealed(&value.source_sha256),
            file_metadata_sha256: metadata_digest(&value.file_metadata_sha256),
            project_inputs_sha256: inputs_digest(&value.project_inputs_sha256),
            expected: wire_findings(&value.expected),
            reference_expected: match &value.reference_expected {
                ExpectedFindings::Derived(rows) => Some(wire_findings(rows)),
                ExpectedFindings::Underivable => None,
            },
            diagnostics: match &value.diagnostics {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedFindings::Reported(rows) => Some(wire_findings(rows)),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedFindings::Unreported => None,
            },
            skipped: match &value.skipped {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedSkips::Reported(rows) => Some(
                    rows.iter()
                        .map(|skip| SkippedSchema {
                            rule: skip.rule.0.clone(),
                            path: skip.path.0.clone(),
                            reason: SkipReasonSchema::SyntaxRejected,
                        })
                        .collect(),
                ),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedSkips::Unreported => None,
            },
            observation_schema: value.observation_schema.0.clone(),
            expectation_contract: wire_contract(value.expectation_contract),
            execution_metadata: match &value.execution_metadata {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedMetadata::Recorded(metadata) => Some(match metadata {
                    ExecutionMetadata::Unknown => ExecutionSchema::Unknown {},
                    ExecutionMetadata::Unavailable { reason } => ExecutionSchema::Unavailable {
                        reason: reason.0.clone(),
                    },
                    ExecutionMetadata::Complete => ExecutionSchema::Complete {},
                }),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedMetadata::Unrecorded => None,
            },
            coverage: match value.coverage {
                CoverageAccounting::Accounted(coverage) => Some(CoverageSchema {
                    selected_rule_file_pairs: coverage.selected.0,
                    evaluated_rule_file_pairs: coverage.evaluated.0,
                    skipped_rule_file_pairs: coverage.skipped.0,
                    unknown_or_unread_rule_file_pairs: coverage.unknown_or_unread.0,
                }),
                CoverageAccounting::Unaccounted => None,
            },
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
            actual_request_root: match &value.actual_request_root {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::RequestRoot::Written(root) => Some(root.0.clone()),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::RequestRoot::Unwritten => None,
            },
            documented_reference_discrepancy: is(value.documented_reference_discrepancy),
            documented_tool_contract_difference: match value.documented_tool_contract_difference {
                ToolContractRecord::Recorded(answer) => Some(is(answer)),
                ToolContractRecord::Unrecorded => None,
            },
            discrepancy_validation: match &value.discrepancy_validation {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ValidationRecord::Attested(proof) => Some({
                ValidationSchema {
                    discrepancy_basis: wire_basis(proof.discrepancy_basis),
                    process_observation_group_sha256: pin_text(
                        &proof.process_observation_group_sha256,
                    ),
                    reference_expectation_kind: proof.reference_expectation_kind.into(),
                    settings_sha256: text(&proof.settings_sha256),
                    actual_executable_sha256: text(&proof.actual_executable_sha256),
                    profile_sha256: text(&proof.profile_sha256),
                    source_sha256: text(&proof.source_sha256),
                    reference_output_sha256: text(&proof.reference_output_sha256),
                    intended_output_sha256: text(&proof.intended_output_sha256),
                    file_metadata_sha256: metadata_digest(&proof.file_metadata_sha256),
                }
                }),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ValidationRecord::Unattested => None,
            },
            raw_parity_matches_intended: match value.raw_parity_matches_intended {
                IntentMatch::Matched(answer) => Some(is(answer)),
                IntentMatch::Unreported => None,
            },
            diagnostics_match_intended: match value.diagnostics_match_intended {
                IntentMatch::Matched(answer) => Some(is(answer)),
                IntentMatch::Unreported => None,
            },
            process_observation: match &value.process_observation {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(process) => Some(process.into()),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::NotObserved => None,
            },
        }
    }
}

/// Encode one observation row as the JSON line appended to a receipt file.
pub(in super::super) fn row_bytes(
    document: &ObservationDocument,
) -> Result<Vec<u8>, ReceiptsError> {
    serde_json::to_vec(&ObservationSchema::from(document))
        .map_err(|error| ReceiptsError::Invalid(error.to_string()))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
