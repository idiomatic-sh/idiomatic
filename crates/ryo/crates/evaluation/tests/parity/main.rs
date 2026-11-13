//! Parity of the fused evaluator with the interpreter, on the engine's Python fixtures.
#![allow(missing_docs, reason = "spike probe")]
use std::sync::Arc;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;

fn ruff() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../languages/python/benchmarks/linters/ruff")
}

#[test]
fn dump() {
    let python = ryo_python::linked_with_settings(ryo_python::settings::PythonParserSettings::new(
        ryo_python::settings::PythonTargetVersion::Py310,
    ));
    let parse = python.parse_engine();
    let vocab = ryo_values::domain::descriptions::types::KindVocabularies(vec![ryo_values::domain::descriptions::types::KindVocabulary {
        language: ryo_values::domain::descriptions::types::LanguageName("python".into()),
        kinds: parse.kind_names().0.into_iter().map(|n| ryo_values::domain::descriptions::types::KindWord(Box::from(n.0))).collect(),
    }]);
    let mut modules = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(ruff().join("fragments")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "ryoql") {
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let id = ryo_values::domain::query_tokens::types::QuerySourceId(format!("python/ruff/{name}").into());
            modules.insert(id.clone(), ryo_values::domain::query_tokens::types::QueryDocumentSource { id, text: std::fs::read_to_string(&path).unwrap().into() });
        }
    }
    let modules = Arc::new(modules);
    let rules: Vec<_> = std::env::var("RULES").unwrap_or("PLE2510,LOG009".into()).split(',').map(|code| ryo_values::domain::rule_requests::types::RuleSpec {
        id: ryo_values::domain::runs::types::RuleId(code.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle {
            root: ryo_values::domain::query_tokens::types::QueryDocumentSource { id: ryo_values::domain::query_tokens::types::QuerySourceId(code.into()), text: std::fs::read_to_string(ruff().join("rules").join(code).join("query.ryoql")).unwrap().into() },
            modules: Arc::clone(&modules),
        },
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: vec![], observations: vec![],
        spelled: ryo_values::domain::rule_requests::types::RuleParameters("".into()),
        include: vec![], exclude: vec![],
    }).collect();
    let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let catalog = Arc::new(ryo_query_language::outbound::catalog::factory::language());
    let compile = Arc::new(ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(syntax, catalog));
    let typecheck = Arc::new(ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let ql = ryo_execution::outbound::query_language::factory::use_case(compile, typecheck);
    let compiler = ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl::new(Arc::new(ql), Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()));
    let compiled = compiler.compile_rules(ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest { rules: &rules, hosted: &[], vocabularies: &vocab }).unwrap();
    for (rule, plan) in rules.iter().zip(compiled.plans()) {
        println!("=== {} steps={} params={} observations={} frontier={:?} emit={:?}", rule.id.0, plan.steps.len(), plan.parameters.len(), plan.observations.len(), plan.frontier, plan.emit);
        for (i, step) in plan.steps.iter().enumerate() {
            println!("{i:3} {:?}", step);
        }
    }
}
