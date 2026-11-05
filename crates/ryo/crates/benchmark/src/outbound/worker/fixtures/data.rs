//! Canonical request and response samples, kept apart from contract rows.
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
/// The request fixture both concrete transports and doubles receive.
#[must_use]
pub fn request() -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(vec![
            ryo::linter::RuleId::try_from("fixture").expect("fixture rule"),
        ]),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: "workspace".into(),
        rules: "rules".into(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
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
        project_context: None,
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        skipped: Vec::new(),
        coverage: Some(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: 1,
                evaluated: 1,
                skipped: 0,
                unknown_or_unread: 0,
            },
        ),
        diagnostics: vec![finding.clone(), finding],
    }
}
