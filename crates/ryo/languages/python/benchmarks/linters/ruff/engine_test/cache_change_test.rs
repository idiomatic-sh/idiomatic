//! A staged query module answers as its inline query does.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::fixture_cohort;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn query_module_inline_equivalence(root: &std::path::Path) -> Result<bool, Error> {
    let rules = root.join("rules");
    let rule_dir = rules.join("fixture");
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(root.join("input.py"), "value = 1\n")?;
    let input = "let nodes = select id, file, span from \"syntax-tree\";\n";
    let inline = format!("{input}emit nodes subject file key(span);\n");
    std::fs::write(rule_dir.join("query.ryoql"), inline)?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked()]);
    let plain = factory::local(languages.clone(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["fixture"])),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.into(),
        rules: rules.clone(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;

    let fragments = root.join("fragments");
    std::fs::create_dir_all(&fragments)?;
    std::fs::write(
        fragments.join("identity.ryoql"),
        "input source: relation<id: node, file: file, span: span>;\noutput relation<id: node, file: file, span: span>;\nlet rows = select id, file, span from source;\nreturn rows;\n",
    )?;
    std::fs::write(
        rule_dir.join("query.ryoql"),
        format!(
            "import identity from \"test/identity\";\n{input}let rows = identity(source=nodes);\nemit rows subject file key(span);\n"
        ),
    )?;
    let mut modules = BTreeMap::new();
    let _previous = modules.insert(
        "test/identity".to_owned(),
        std::path::PathBuf::from("fragments/identity.ryoql"),
    );
    let imported = factory::local(languages, None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["fixture"])),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.into(),
        rules,
        query_modules: super::prepared_engine_test::module_names(modules),
        rule_applicability: BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    Ok(plain == imported)
}
