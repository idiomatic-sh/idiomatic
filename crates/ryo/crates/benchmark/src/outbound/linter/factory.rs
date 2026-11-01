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
) -> impl super::interface::LinterPort {
    ryo_context(
        executable,
        rules,
        query_modules,
        rule_applicability,
        parser_configuration,
        crate::domain::suite::types::ProjectContextMode::Isolated,
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
    project_context_mode: crate::domain::suite::types::ProjectContextMode,
) -> impl super::interface::LinterPort {
    LinterRyoAdapter {
        project_context_mode,
        executable,
        rules,
        query_modules,
        rule_applicability,
        parser_configuration,
    }
}
