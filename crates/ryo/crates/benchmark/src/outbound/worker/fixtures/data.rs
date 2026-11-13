//! Canonical request and response samples, kept apart from contract rows.
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
/// The request fixture both concrete transports and doubles receive.
///
/// # Panics
/// Never: the rule identifier is a valid literal.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the rule identifier is a valid literal, so its parse cannot fail"
)]
pub fn request() -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            vec![ryo::linter::RuleId::try_from("fixture").expect("fixture rule")],
        ),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "workspace".into(),
        rules: "rules".into(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            "cache".into(),
        ),
    }
}
/// A report with duplicated findings, which encoding must preserve.
///
/// # Panics
///
/// Panics if the fixed fixture rule identifier is no longer accepted.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the literal fixture rule must be valid for this contract"
)]
pub fn report() -> RunReport {
    let finding = BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("fixture").expect("fixture rule"),
        file: ryo::linter::RelativePath("source.txt".into()),
        start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(2),
        end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(4),
    };
    RunReport {
        execution_metadata:
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
        project_context:
            ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        skipped: Vec::new(),
        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            },
        ),
        diagnostics: vec![finding.clone(), finding],
    }
}
