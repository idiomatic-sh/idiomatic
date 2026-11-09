//! Sample requests, cases, inventories and observation rows the suite contract and its fake run over.

/// A request that declares no file metadata.
pub fn request(
    root: &std::path::Path,
) -> ryo_benchmark_contracts::domain::requests::types::RunRequest {
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
        RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
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

/// The tool a suite adapter is exercised with: one default rule, everything else undeclared.
pub fn tool() -> ryo_benchmark_contracts::domain::workloads::types::Tool {
    ryo_benchmark_contracts::domain::workloads::types::Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context:
            ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
        file_observation:
            ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: Default::default(),
        pending: Default::default(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    }
}
