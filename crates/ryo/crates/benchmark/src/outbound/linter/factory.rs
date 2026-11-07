//! Builds the linter adapters from dependencies chosen by the composition.
use super::adapters::ryo::adapter::LinterRyoAdapter;
/// Construct the ryo implementation of the linter contract.
#[must_use]
pub fn ryo(
    executable: std::path::PathBuf,
    rules: std::path::PathBuf,
    query_modules: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::QueryModuleName, std::path::PathBuf>,
    rule_applicability: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::RuleLabel, ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy>,
    parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection,
    defaults: Vec<ryo::linter::RuleId>,
    rule_observations: ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
) -> impl super::interface::LinterPort {
    ryo_context(
        executable,
        rules,
        query_modules,
        rule_applicability,
        parser_configuration,
        ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::Isolated,
        defaults,
        ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        rule_observations,
    )
}

/// Construct a candidate whose project context is explicitly part of its profile identity.
#[must_use]
pub fn ryo_context(
    executable: std::path::PathBuf,
    rules: std::path::PathBuf,
    query_modules: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::QueryModuleName, std::path::PathBuf>,
    rule_applicability: std::collections::BTreeMap<ryo_benchmark_contracts::domain::requests::types::RuleLabel, ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy>,
    parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection,
    source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection,
    project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode,
    defaults: Vec<ryo::linter::RuleId>,
    rule_observation_selection: ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy,
    rule_observations: ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
) -> impl super::interface::LinterPort {
    LinterRyoAdapter {
        rule_observation_selection,
        defaults,
        rule_observations,
        project_context_mode,
        executable,
        rules,
        query_modules,
        rule_applicability,
        parser_configuration,
        source_language,
    }
}
