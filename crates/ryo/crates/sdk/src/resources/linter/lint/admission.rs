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
        .or_else(|refused| match &refused {
            // A relation a linked language declares (its own named file relations) is not in the compiler's catalog: the executor binds it.
            LintError::Rule { error, .. } if unknown_relation(error).is_some_and(|name| declared(engine, name)) => Ok(()),
            _ => Err(refused),
        })
}

/// The relation a query refusal says no catalog has.
fn unknown_relation(error: &crate::domain::query::errors::ValidateError) -> Option<&str> {
    use crate::domain::query::errors::ValidateError;
    match error {
        ValidateError::Located { error, .. } => unknown_relation(error),
        ValidateError::UnknownRelation { name, .. } => Some(name),
        _ => None,
    }
}

/// Whether a linked language declares a file relation of that name.
fn declared(engine: &LinterLintEngine, name: &str) -> bool {
    engine.languages.0.iter().any(|language| {
        language.parse_use_case().file_relations().is_some_and(|supplier| supplier.declarations().iter().any(|relation| relation.name == name))
    })
}
