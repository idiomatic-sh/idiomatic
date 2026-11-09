//! The profile and case samples the linter contract rows read, kept apart from the rows.
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, ProjectContext, RunReport,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, ParserSelection,
    ProviderSelection, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, FileObservation, PreparedCase, PreparedReferenceBehavior, RuleContextProfile,
    Tool,
};
use std::collections::BTreeMap;

/// A profile with no declarations: the settings text depends only on the invocation and the case.
#[must_use]
pub fn tool() -> Tool {
    Tool {
        rule_observation_selection: ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: RuleContextProfile::Undeclared,
        file_observation: FileObservation::Undeclared,
        project_context_mode: ProjectContextMode::Isolated,
        tool: "fixture".into(),
        version: "1.0".into(),
        sha256: Vec::new(),
        defaults: Vec::new(),
        deferred: BTreeMap::default(),
        pending: BTreeMap::default(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: ParserSelection::Unspecified,
        source_language: ProviderSelection::Unspecified,
        project_search_roots: SearchRoots::Undeclared,
    }
}

/// A prepared case whose reference agrees with the intended behavior.
///
/// # Panics
///
/// Panics if the fixed fixture rule identifier is no longer accepted.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the literal fixture rule must be valid for this contract"
)]
pub fn case() -> PreparedCase {
    PreparedCase {
        comparison: CaseComparison::Full,
        rule: ryo::linter::RuleId::try_from("F821").expect("valid ID"),
        name: "settings-sample".into(),
        request: RunRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Frozen,
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            staged_sources: StagedSources::Unlisted,
            file_metadata: FileMetadataDeclaration::Undeclared,
            context_manifest: ContextSeal::Unsealed,
            root: ".".into(),
            state: "cache".into(),
            files: vec![ryo::linter::RelativePath("input.py".into())],
        },
        expected: RunReport {
            execution_metadata: ExecutionMetadata::Complete,
            project_context: ProjectContext::NotRequested,
            diagnostics: Vec::new(),
            skipped: Vec::new(),
            coverage: CoverageAccounting::Unaccounted,
            files: vec![ryo::linter::RelativePath("input.py".into())],
        },
        reference_discrepancy: PreparedReferenceBehavior::Agrees,
    }
}
