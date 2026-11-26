//! The checks the old lint use case made before it ran anything, here where a request is admitted now: before the executor is called,
//! for every request of a list, each answered in its own slot. They are the old checks, moved: the source language a request names
//! must be one the linter has (`SourceLanguage`), and the rules must compile (`Rule` for a query that is not correct RyoQL,
//! `MissingParameter` for a parameter nothing is bound to, `Unrunnable` for a rule whose query cannot run, among them a word for a kind
//! no linked language has).

use super::errors::LintError;
use super::schemas;
use super::types::{LintRequest, LinterLintEngine};
use crate::domain::language::types::SourceLanguage;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;

/// Whether `request` may run: the failure it is answered with when it may not.
pub(super) fn admit(request: &LintRequest, engine: &LinterLintEngine) -> Result<(), LintError> {
    if let SourceLanguage::Named(selected) = &request.source_language
        && engine.languages.0.iter().filter(|language| language.name() == selected).count() != 1
    {
        return Err(LintError::SourceLanguage { name: selected.0.to_string() });
    }
    let rules = schemas::core_rules(request.rules.clone());
    let hosted = schemas::host_relations(request.hosted.clone())?;
    let specs: Vec<_> = rules.iter().map(ryo_values::domain::rule_requests::types::RuleSpec::from).collect();
    engine
        .compile
        .compile_rules(CompileRequest { rules: &specs, hosted: &hosted, vocabularies: &engine.vocabularies })
        .map(drop)
        .map_err(|error| LintError::from_core(error.into()))
}
