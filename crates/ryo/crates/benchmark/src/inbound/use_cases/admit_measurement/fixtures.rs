//! The audit and tool one admission is exercised against, through the real audit operation.
use ryo_benchmark_contracts::domain::workloads::types::Tool;

/// A tool declaring the one default rule the audit fixture authors.
///
/// # Panics
///
/// Panics when the fixed fixture rule identifier is no longer accepted.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the literal fixture rule must be valid for this fixture"
)]
pub fn tool() -> Tool {
    Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
        file_observation: ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    }
}
