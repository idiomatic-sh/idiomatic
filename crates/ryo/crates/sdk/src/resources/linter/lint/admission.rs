//! The checks the old lint use case made before it ran anything, here where a request is admitted now: before the executor is called,
//! for every request of a list, each answered in its own slot. They are the old checks, moved: the source language a request names
//! must be one the linter has (`SourceLanguage`), and the rules must compile (`Rule` for a query that is not correct RyoQL,
//! `MissingParameter` for a parameter nothing is bound to, `Unrunnable` for a rule whose query cannot run, among them a word for a kind
//! no linked language has).

use super::errors::LintError;
use super::schemas;
use super::types::{Admitted, LintRequest, LinterLintEngine, RuleId};
use crate::domain::query::types::RyoQlQuery;
use crate::domain::language::types::SourceLanguage;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_values::domain::observations::types::{ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName};

/// Whether `request` may run: the failure it is answered with when it may not.
pub static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static COMPILED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(super) fn admit(request: &LintRequest, engine: &LinterLintEngine) -> Result<(), LintError> {
    let _ = ASKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let SourceLanguage::Named(selected) = &request.source_language
        && engine.languages.0.iter().filter(|language| language.name() == selected).count() != 1
    {
        return Err(LintError::SourceLanguage { name: selected.0.to_string() });
    }
    // The rules are compiled once for each distinct set of them (and of host relations): a list of thousands of requests over a few rule
    // sets is admitted by a few compiles, the rest by a lookup.
    let key: Vec<(RuleId, RyoQlQuery)> = request.rules.iter().map(|(id, rule)| (id.clone(), rule.query.clone())).collect();
    {
        let memo = engine.admitted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(known) = memo.get(&key).and_then(|sets| sets.iter().find(|set| set.rules == request.rules && set.hosted == request.hosted)) {
            return known.answer.clone();
        }
    }
    let _ = COMPILED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let answer = compiled(request, engine);
    engine
        .admitted
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(key)
        .or_default()
        .push(Admitted { rules: request.rules.clone(), hosted: request.hosted.clone(), answer: answer.clone() });
    answer
}

/// What compiling the rules answers: the old compile, over the shared catalog and the relations the languages declare.
fn compiled(request: &LintRequest, engine: &LinterLintEngine) -> Result<(), LintError> {
    let mut rules = schemas::core_rules(request.rules.clone());
    let declared = declared(engine);
    for rule in &mut rules {
        for binding in &declared {
            if !rule.observations.iter().any(|own| own.name == binding.name) {
                rule.observations.push(binding.clone());
            }
        }
    }
    let hosted = schemas::host_relations(request.hosted.clone())?;
    let specs: Vec<_> = rules.iter().map(ryo_values::domain::rule_requests::types::RuleSpec::from).collect();
    engine
        .compile
        .compile_rules(CompileRequest { rules: &specs, hosted: &hosted, vocabularies: &engine.vocabularies })
        .map(drop)
        .map_err(|error| LintError::from_core(error.into()))
}

/// The relations the linked languages declare themselves (their named file relations), as the inputs a query reads beside the shared
/// catalog's: the compile knows each by its name and columns, so a rule over one is admitted and a name nobody declares is refused as before.
fn declared(engine: &LinterLintEngine) -> Vec<ObservationBinding> {
    let mut found: Vec<ObservationBinding> = Vec::new();
    for language in &engine.languages.0 {
        let parser = language.parse_use_case();
        let Some(supplier) = parser.file_relations() else { continue };
        for relation in supplier.declarations() {
            if found.iter().any(|known| &*known.name.0 == relation.name) {
                continue;
            }
            found.push(ObservationBinding {
                name: ObservationBindingName(relation.name.into()),
                columns: relation
                    .columns
                    .iter()
                    .map(|(name, kind)| ObservationColumn { name: ObservationColumnName((*name).into()), kind: *kind })
                    .collect(),
                producers: Vec::new(),
            });
        }
    }
    found
}
