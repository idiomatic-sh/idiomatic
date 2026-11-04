//! Canonical request and response samples, kept apart from contract rows.
use crate::domain::{
    diagnostics::types::{BenchmarkFinding, RunReport},
    suite::types::WorkerRequest,
};
/// The request fixture both concrete transports and doubles receive.
#[must_use]
pub fn request() -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        context_manifest: None,
        root: "workspace".into(),
        rules: "rules".into(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        state: crate::domain::suite::types::WorkerState::Retained("cache".into()),
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
        start: crate::domain::diagnostics::types::SourceByteOffset(2),
        end: crate::domain::diagnostics::types::SourceByteOffset(4),
    };
    RunReport {
        execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
        project_context: None,
        files: vec![ryo::linter::RelativePath("source.txt".into())],
        skipped: Vec::new(),
        coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        diagnostics: vec![finding.clone(), finding],
    }
}
