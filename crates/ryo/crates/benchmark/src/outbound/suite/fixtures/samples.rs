//! Sample requests, cases, inventories and observation rows the suite contract and its fake run over.
use ryo_benchmark_contracts::domain::documents::types::observation_document::ObservationDocument;

/// A request that declares no file metadata.
pub fn request(root: &std::path::Path) -> ryo_benchmark_contracts::domain::requests::types::RunRequest {
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, RuleContextPreparation,
        RunRequest, SearchRoots, StagedSources,
    };
    RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: root.to_path_buf(),
        files: Vec::new(),
        state: root.to_path_buf(),
    }
}

/// A case whose only source path leaves the directory it is materialized in.
pub fn escaping_case() -> ryo_benchmark_contracts::domain::workloads::types::Case {
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, FileMetadataDeclaration, InputDeclaration, SearchRoots,
    };
    use ryo_benchmark_contracts::domain::workloads::types::{
        Case, CaseComparison, ConfigurationOverride, ContextDeclaration, InventoryDeclaration,
        ReferenceBehavior, RequestedFiles,
    };
    Case {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: ConfigurationOverride::Inherited,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: InventoryDeclaration::Undeclared,
        requested_files: RequestedFiles::Legacy,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context: ContextDeclaration::Undeclared,
        comparison: CaseComparison::Rule,
        name: String::from("escape").into(),
        files: std::iter::once((
            ryo::linter::RelativePath("../escape.py".into()),
            ryo::linter::SourceText("x".into()),
        ))
        .collect(),
        expected: Vec::new(),
        reference_discrepancy: ReferenceBehavior::Agrees,
    }
}

/// A Given inventory that names a file nothing staged.
pub fn given_file() -> ryo::linter::GivenProjectInventory {
    ryo::linter::GivenProjectInventory {
        entries: vec![ryo::linter::ProjectEntryObservation {
            path: ryo::linter::RelativePath("absent.py".into()),
            state: ryo::linter::ProjectEntryState::File,
        }],
        ..Default::default()
    }
}

/// One complete observation row of a native side.
pub fn observation() -> ObservationDocument {
use ryo_benchmark_contracts::domain::documents::types::{observation_document::{
            ContextRun, FileMetadataDigest, IntentMatch, ProjectContextRecord,
            ProjectInputsDigest, ReferenceExpectation, RuleCohort, RuleContextRecord,
            RuleObservationsRecord, SourceIdentity, ToolContractRecord,
        }, text::{CaseName, ContestantSide, RuleName, SchemaName, YesNo}};
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    ObservationDocument {
        rule: RuleName("R1".into()),
        name: CaseName("one".into()),
        side: ContestantSide::Reference,
        complete: YesNo::Yes,
        error: None,
        case_passed: YesNo::Yes,
        files: Vec::new(),
        source_sha256: SourceIdentity::Unavailable,
        file_metadata_sha256: FileMetadataDigest::Undeclared,
        project_inputs_sha256: ProjectInputsDigest::Undeclared,
        expected: Vec::new(),
        reference_expected: ReferenceExpectation::Underivable,
        diagnostics: None,
        skipped: None,
        observation_schema: SchemaName("benchmark-case/v2".into()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: None,
        coverage: None,
        context_manifest: ContextRun::Isolated,
        project_context: ProjectContextRecord::NotCarried,
        rule_context: RuleContextRecord::Undeclared,
        rule_observations: RuleObservationsRecord::Undeclared,
        selected_rules: RuleCohort::Undeclared,
        actual_request_root: None,
        documented_reference_discrepancy: YesNo::No,
        documented_tool_contract_difference: ToolContractRecord::Unrecorded,
        discrepancy_validation: None,
        raw_parity_matches_intended: None,
        diagnostics_match_intended: IntentMatch::Unreported,
        process_observation: None,
    }
}

