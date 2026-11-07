//! Sample observation rows the receipts contract and its tests run over.
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;

/// One complete observation row of a native side.
pub fn observation() -> ObservationDocument {
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ContextRun, IntentMatch, ProjectContextRecord, RuleCohort, RuleContextRecord, RuleObservationsRecord, SourceIdentity, ToolContractRecord};
use ryo_benchmark_contracts::domain::documents::types::text::value::{CaseName, ContestantSide, FileMetadataDigest, ProjectInputsDigest, RuleName, SchemaName, YesNo};
use ryo_benchmark_contracts::domain::documents::types::finding::value::ExpectedFindings;
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    ObservationDocument {
        rule: RuleName("R1".into()),
        name: CaseName("one".into()),
        side: ContestantSide::Reference,
        complete: YesNo::Yes,
        error: ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedReason::Absent,
        case_passed: YesNo::Yes,
        files: Vec::new(),
        source_sha256: SourceIdentity::Unavailable,
        file_metadata_sha256: FileMetadataDigest::Undeclared,
        project_inputs_sha256: ProjectInputsDigest::Undeclared,
        expected: Vec::new(),
        reference_expected: ExpectedFindings::Underivable,
        diagnostics: ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedFindings::Unreported,
        skipped: ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReportedSkips::Unreported,
        observation_schema: SchemaName("benchmark-case/v2".into()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: ryo_benchmark_contracts::domain::documents::types::observation_document::value::RecordedMetadata::Unrecorded,
        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted,
        context_manifest: ContextRun::Isolated,
        project_context: ProjectContextRecord::NotCarried,
        rule_context: RuleContextRecord::Undeclared,
        rule_observations: RuleObservationsRecord::Undeclared,
        selected_rules: RuleCohort::Undeclared,
        actual_request_root: ryo_benchmark_contracts::domain::documents::types::observation_document::value::RequestRoot::Unwritten,
        documented_reference_discrepancy: YesNo::No,
        documented_tool_contract_difference: ToolContractRecord::Unrecorded,
        discrepancy_validation: ryo_benchmark_contracts::domain::documents::types::observation_document::value::ValidationRecord::Unattested,
        raw_parity_matches_intended: ryo_benchmark_contracts::domain::documents::types::observation_document::value::IntentMatch::Unreported,
        diagnostics_match_intended: IntentMatch::Unreported,
        process_observation: ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::NotObserved,
    }
}

