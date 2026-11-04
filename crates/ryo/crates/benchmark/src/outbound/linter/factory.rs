//! Builds the linter adapters from dependencies chosen by the composition.
use super::adapters::ryo::adapter::LinterRyoAdapter;
/// Construct the ryo implementation of the linter contract.
#[must_use]
pub fn ryo(
    executable: std::path::PathBuf,
    rules: std::path::PathBuf,
    query_modules: std::collections::BTreeMap<String, std::path::PathBuf>,
    rule_applicability: std::collections::BTreeMap<String, String>,
    parser_configuration: Option<String>,
    defaults: Vec<ryo::linter::RuleId>,
    rule_observations: crate::domain::suite::types::RuleObservationDeclarations,
) -> impl super::interface::LinterPort {
    ryo_context(
        executable,
        rules,
        query_modules,
        rule_applicability,
        parser_configuration,
        None,
        crate::domain::suite::types::ProjectContextMode::Isolated,
        defaults,
        rule_observations,
    )
}

/// Construct a candidate whose project context is explicitly part of its profile identity.
#[must_use]
pub fn ryo_context(
    executable: std::path::PathBuf,
    rules: std::path::PathBuf,
    query_modules: std::collections::BTreeMap<String, std::path::PathBuf>,
    rule_applicability: std::collections::BTreeMap<String, String>,
    parser_configuration: Option<String>,
    source_language: Option<String>,
    project_context_mode: crate::domain::suite::types::ProjectContextMode,
    defaults: Vec<ryo::linter::RuleId>,
    rule_observations: crate::domain::suite::types::RuleObservationDeclarations,
) -> impl super::interface::LinterPort {
    LinterRyoAdapter {
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
