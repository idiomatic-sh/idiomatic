//! The independently authored case and profile the suite fake serves to orchestration tests.
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
    RuleCoverage, RunReport, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, ParserSelection,
    ProviderSelection, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, FileObservation, PreparedCase, PreparedReferenceBehavior, RuleContextProfile,
    Tool,
};

/// The complete one-file report with `diagnostics`.
fn report(diagnostics: Vec<BenchmarkFinding>) -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        files: vec![ryo::linter::RelativePath("case.py".into())],
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        diagnostics,
    }
}

/// A finding of `rule` over `start..end` bytes of the case file.
pub fn finding(rule: &str, start: usize, end: usize) -> BenchmarkFinding {
    BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from(rule).expect("fixture rule identity"),
        file: ryo::linter::RelativePath("case.py".into()),
        start: SourceByteOffset(start),
        end: SourceByteOffset(end),
    }
}

/// The authored case `positive` of rule `E722`, which expects `expected`.
pub fn case(expected: &BenchmarkFinding) -> PreparedCase {
    PreparedCase {
        reference_discrepancy: PreparedReferenceBehavior::Agrees,
        comparison: CaseComparison::Rule,
        rule: ryo::linter::RuleId::try_from("E722").unwrap(),
        name: "positive".into(),
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
            files: vec![ryo::linter::RelativePath("case.py".into())],
            state: "cache".into(),
        },
        expected: report(vec![expected.clone()]),
    }
}

/// The profile pinning the fixture tool, whose defaults are `E722` and `F901`.
pub fn profile() -> Tool {
    Tool {
        rule_observation_selection: ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: RuleContextProfile::Undeclared,
        file_observation: FileObservation::Undeclared,
        project_context_mode: ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: ParserSelection::Unspecified,
        source_language: ProviderSelection::Unspecified,
        project_search_roots: SearchRoots::Undeclared,
    }
}
