//! The shape of the native receipt rows a receipt file holds, and their checked conversion into the core's observation documents.
//! Unknown fields are refused; the documents are a contract.
use super::super::process_row::schemas::{KindSchema, ProcessRowSchema, pin};
use crate::outbound::receipts::errors::ReceiptsError;
use ryo_benchmark_contracts::domain::documents::types::finding::value::{
    ExpectedFindings, FindingOffset, RecordedFinding, RecordedSpan, SkippedPair,
};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ContextMode, ContextRun, IntentMatch, ListedContext, ObservationDocument, RecordedMetadata,
    RecordedReason, ReferenceProcess, ReportedFindings, ReportedSkips, RequestRoot,
    ProjectContextRecord, RuleCohort, RuleContextRecord, RuleObservationsRecord, SourceIdentity,
    ToolContractRecord, ValidationRecord,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::DiscrepancyValidation;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, CaseName, ContestantSide, FileMetadataDigest, PathText, ProjectInputsDigest,
    ReasonText, RuleName, SchemaName, Sha256Text, YesNo,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, CoverageAccounting, ExecutionMetadata, PairCount, RuleCoverage,
    UnavailableReason,
};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::Deserialize;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// One diagnostic as a row records it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingSchema {
    rule: String,
    file: String,
    start: u64,
    end: u64,
}

/// Which contestant a row belongs to.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SideSchema {
    Reference,
    Candidate,
}

/// Why a rule/file pair was skipped.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}

/// One rule/file pair skipped after the parser rejected the file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkippedSchema {
    rule: String,
    path: String,
    reason: SkipReasonSchema,
}

/// What a producer can establish about execution.
#[derive(Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
enum ExecutionSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}

/// Rule/file accounting of a report.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageSchema {
    selected_rule_file_pairs: usize,
    evaluated_rule_file_pairs: usize,
    skipped_rule_file_pairs: usize,
    unknown_or_unread_rule_file_pairs: usize,
}

/// How a context was staged.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}

/// The listed context a side actually ran in.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextSchema {
    mode: ModeSchema,
    logical_root: String,
    actual_root: String,
    manifest_hash: String,
}

/// The evidence a row was compared under.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContractSchema {
    Diagnostics,
    CompleteReport,
}

/// The authority a validated discrepancy rests on.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BasisSchema {
    LanguageDefect,
    ToolContractDifference,
}

/// What a native side attests it validated for a discrepancy.
#[derive(Deserialize)]
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
    #[serde(default)]
    file_metadata_sha256: Option<String>,
}

/// One native side's receipt row.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RowSchema {
    rule: String,
    name: String,
    side: SideSchema,
    complete: bool,
    error: Option<String>,
    case_passed: bool,
    files: Vec<String>,
    source_sha256: Option<String>,
    #[serde(default)]
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
    context_manifest: Option<ContextSchema>,
    project_context: Option<serde_json::Value>,
    #[serde(default)]
    rule_context: Option<serde_json::Value>,
    #[serde(default)]
    rule_observations: Option<serde_json::Value>,
    #[serde(default)]
    selected_rules: Option<Vec<String>>,
    actual_request_root: Option<String>,
    documented_reference_discrepancy: bool,
    documented_tool_contract_difference: Option<bool>,
    discrepancy_validation: Option<ValidationSchema>,
    raw_parity_matches_intended: Option<bool>,
    diagnostics_match_intended: Option<bool>,
    #[serde(default)]
    process_observation: Option<ProcessRowSchema>,
}

fn findings(rows: Vec<FindingSchema>) -> Vec<RecordedFinding> {
    rows.into_iter()
        .map(|row| RecordedFinding {
            rule: RuleName(row.rule),
            file: PathText(row.file),
            span: RecordedSpan {
                start: FindingOffset(row.start),
                end: FindingOffset(row.end),
            },
        })
        .collect()
}

fn reason(value: Option<String>) -> RecordedReason {
    value.map_or(RecordedReason::Absent, |text| RecordedReason::Stated(ReasonText(text)))
}

fn intent(value: Option<bool>) -> IntentMatch {
    value.map_or(IntentMatch::Unreported, |answer| IntentMatch::Matched(YesNo::from(answer)))
}

fn canonical(value: serde_json::Value) -> CanonicalJson {
    CanonicalJson(value.to_string())
}

impl From<ValidationSchema> for DiscrepancyValidation {
    fn from(schema: ValidationSchema) -> Self {
        Self {
            discrepancy_basis: match schema.discrepancy_basis {
                BasisSchema::LanguageDefect => DiscrepancyBasis::LanguageDefect,
                BasisSchema::ToolContractDifference => DiscrepancyBasis::ToolContract,
            },
            process_observation_group_sha256: pin(schema.process_observation_group_sha256),
            reference_expectation_kind: schema.reference_expectation_kind.into(),
            settings_sha256: Sha256Text(schema.settings_sha256),
            actual_executable_sha256: Sha256Text(schema.actual_executable_sha256),
            profile_sha256: Sha256Text(schema.profile_sha256),
            source_sha256: Sha256Text(schema.source_sha256),
            reference_output_sha256: Sha256Text(schema.reference_output_sha256),
            intended_output_sha256: Sha256Text(schema.intended_output_sha256),
            file_metadata_sha256: FileMetadataDigest::from(
                schema.file_metadata_sha256.map(Sha256Text),
            ),
        }
    }
}

impl From<RowSchema> for ObservationDocument {
    fn from(schema: RowSchema) -> Self {
        Self {
            rule: RuleName(schema.rule),
            name: CaseName(schema.name),
            side: match schema.side {
                SideSchema::Reference => ContestantSide::Reference,
                SideSchema::Candidate => ContestantSide::Candidate,
            },
            complete: YesNo::from(schema.complete),
            error: reason(schema.error),
            case_passed: YesNo::from(schema.case_passed),
            files: schema.files.into_iter().map(PathText).collect(),
            source_sha256: schema
                .source_sha256
                .map_or(SourceIdentity::Unavailable, |digest| SourceIdentity::Sealed(Sha256Text(digest))),
            file_metadata_sha256: FileMetadataDigest::from(schema.file_metadata_sha256.map(Sha256Text)),
            project_inputs_sha256: ProjectInputsDigest::from(schema.project_inputs_sha256.map(Sha256Text)),
            expected: findings(schema.expected),
            reference_expected: schema
                .reference_expected
                .map_or(ExpectedFindings::Underivable, |rows| ExpectedFindings::Derived(findings(rows))),
            diagnostics: schema
                .diagnostics
                .map_or(ReportedFindings::Unreported, |rows| ReportedFindings::Reported(findings(rows))),
            skipped: schema.skipped.map_or(ReportedSkips::Unreported, |rows| {
                ReportedSkips::Reported(
                    rows.into_iter()
                        .map(|skip| {
                            let SkipReasonSchema::SyntaxRejected = skip.reason;
                            SkippedPair {
                                rule: RuleName(skip.rule),
                                path: PathText(skip.path),
                            }
                        })
                        .collect(),
                )
            }),
            observation_schema: SchemaName(schema.observation_schema),
            expectation_contract: match schema.expectation_contract {
                ContractSchema::Diagnostics => ComparisonContract::Diagnostics,
                ContractSchema::CompleteReport => ComparisonContract::CompleteReport,
            },
            execution_metadata: schema.execution_metadata.map_or(RecordedMetadata::Unrecorded, |metadata| {
                RecordedMetadata::Recorded(match metadata {
                    ExecutionSchema::Unknown {} => ExecutionMetadata::Unknown,
                    ExecutionSchema::Unavailable { reason } => ExecutionMetadata::Unavailable {
                        reason: UnavailableReason(reason),
                    },
                    ExecutionSchema::Complete {} => ExecutionMetadata::Complete,
                })
            }),
            coverage: schema.coverage.map_or(CoverageAccounting::Unaccounted, |coverage| {
                CoverageAccounting::Accounted(RuleCoverage {
                    selected: PairCount(coverage.selected_rule_file_pairs),
                    evaluated: PairCount(coverage.evaluated_rule_file_pairs),
                    skipped: PairCount(coverage.skipped_rule_file_pairs),
                    unknown_or_unread: PairCount(coverage.unknown_or_unread_rule_file_pairs),
                })
            }),
            context_manifest: schema.context_manifest.map_or(ContextRun::Isolated, |context| {
                ContextRun::Listed(ListedContext {
                    mode: match context.mode {
                        ModeSchema::Isolated => ContextMode::Isolated,
                        ModeSchema::ListedDisk => ContextMode::ListedDisk,
                    },
                    logical_root: PathText(context.logical_root),
                    actual_root: PathText(context.actual_root),
                    manifest_hash: Sha256Text(context.manifest_hash),
                })
            }),
            project_context: schema
                .project_context
                .map_or(ProjectContextRecord::NotCarried, |value| ProjectContextRecord::Carried(canonical(value))),
            rule_context: schema
                .rule_context
                .map_or(RuleContextRecord::Undeclared, |value| RuleContextRecord::Declared(canonical(value))),
            rule_observations: schema.rule_observations.map_or(RuleObservationsRecord::Undeclared, |value| {
                RuleObservationsRecord::Declared(canonical(value))
            }),
            selected_rules: schema.selected_rules.map_or(RuleCohort::Undeclared, |rules| {
                RuleCohort::Declared(rules.into_iter().map(RuleName).collect())
            }),
            actual_request_root: schema
                .actual_request_root
                .map_or(RequestRoot::Unwritten, |root| RequestRoot::Written(PathText(root))),
            documented_reference_discrepancy: YesNo::from(schema.documented_reference_discrepancy),
            documented_tool_contract_difference: schema
                .documented_tool_contract_difference
                .map_or(ToolContractRecord::Unrecorded, |answer| {
                    ToolContractRecord::Recorded(YesNo::from(answer))
                }),
            discrepancy_validation: schema
                .discrepancy_validation
                .map_or(ValidationRecord::Unattested, |proof| ValidationRecord::Attested(proof.into())),
            raw_parity_matches_intended: intent(schema.raw_parity_matches_intended),
            diagnostics_match_intended: intent(schema.diagnostics_match_intended),
            process_observation: schema
                .process_observation
                .map_or(ReferenceProcess::NotObserved, |process| ReferenceProcess::Observed(process.into())),
        }
    }
}

/// Read every row of one receipt file; malformed rows and fields the schema does not name are refused.
pub(in super::super) fn rows(text: &str) -> Result<Vec<ObservationDocument>, ReceiptsError> {
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str::<RowSchema>(line)
                .map(ObservationDocument::from)
                .map_err(|error| ReceiptsError::Invalid(format!("receipt row {}: {error}", index + 1)))
        })
        .collect()
}
