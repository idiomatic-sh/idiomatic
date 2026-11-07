//! The shared suppression fragment answers like the historical inline queries.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::suppress_modules;
use super::rule_fixtures_test::write_suppress_module;
use ryo_benchmark::inbound::use_cases::prepare_cases::interface::PrepareCasesUseCase as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn case_identities(
    cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
) -> BTreeMap<(String, String), ryo_benchmark_contracts::domain::workloads::types::PreparedCase> {
    cases
        .into_iter()
        .map(|case| ((case.rule.as_ref().to_owned(), case.name.0.clone()), case))
        .collect()
}

pub(super) fn write_rule_profile(
    rules: &std::path::Path,
    rule: &str,
    query: &str,
    cases: &str,
) -> Result<(), Error> {
    let directory = rules.join(rule);
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("query.ryoql"), query)?;
    std::fs::write(directory.join("cases.toml"), cases)?;
    Ok(())
}

pub(super) fn shared_suppression_fragment_matches_the_historical_inline_queries()
-> Result<(), Error> {
    // E722 retains its historical inline-query baseline. F632 independently exercises
    // inline suppression with current operator-ID semantics: distinct chained operators
    // remain distinct findings even when they share the same source span.
    let temporary = tempfile::tempdir()?;
    let inline_profile = temporary.path().join("inline");
    let imported_profile = temporary.path().join("imported");
    let inline_rules = inline_profile.join("rules");
    let imported_rules = imported_profile.join("rules");
    let fixtures = [
        (
            "E722",
            include_str!("../../../../tests/fixtures/ryoql-fragments/E722/query.ryoql"),
            include_str!("../rules/E722/query.ryoql"),
            include_str!("../rules/E722/cases.toml"),
        ),
        (
            "F632",
            include_str!("../../../../tests/fixtures/ryoql-fragments/F632/query.ryoql"),
            include_str!("../rules/F632/query.ryoql"),
            include_str!("../rules/F632/cases.toml"),
        ),
    ];
    for (rule, baseline, imported, cases) in fixtures {
        write_rule_profile(&inline_rules, rule, baseline, cases)?;
        write_rule_profile(&imported_rules, rule, imported, cases)?;
    }
    write_suppress_module(&imported_profile)?;

    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inline_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
            &ryo_benchmark::outbound::context::factory::local(),
        )
        .prepare_cases(&inline_rules, &temporary.path().join("inline-cases"))?;
    let imported_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
            &ryo_benchmark::outbound::context::factory::local(),
        )
        .prepare_cases(&imported_rules, &temporary.path().join("imported-cases"))?;
    let imported_by_name = case_identities(imported_cases);
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let registry = suppress_modules();
    for case in inline_cases {
        let key = (case.rule.as_ref().to_owned(), case.name.0.clone());
        let imported = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported counterpart for {key:?}")))?;
        write_suppress_module(&imported.request.root)?;
        let state = ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh;
        let inline = inline_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["E722", "F632"])),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: case.request.root,
            rules: inline_rules.clone(),
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            files: case.request.files,
            state,
        })?;
        let fragmented = imported_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["E722", "F632"])),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: imported.request.root.clone(),
            rules: imported_rules.clone(),
            query_modules: super::prepared_engine_test::module_names(registry.clone()),
            rule_applicability: BTreeMap::default(),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            files: imported.request.files.clone(),
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        assert_eq!(
            inline, fragmented,
            "inline and fragment reports differ for {}/{}",
            key.0, key.1
        );
    }
    for (rule, baseline, imported, cases) in fixtures {
        assert_single_rule_fragment_equivalence(temporary.path(), rule, baseline, imported, cases)?;
    }
    Ok(())
}

pub(super) fn assert_single_rule_fragment_equivalence(
    root: &std::path::Path,
    rule: &str,
    baseline: &str,
    imported: &str,
    cases: &str,
) -> Result<(), Error> {
    let inline_rules = root.join(format!("single-inline/{rule}"));
    let imported_rules = root.join(format!("single-imported/{rule}"));
    for (directory, query) in [(&inline_rules, baseline), (&imported_rules, imported)] {
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
        std::fs::write(directory.join("cases.toml"), cases)?;
    }
    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inline_parent = inline_rules
        .parent()
        .ok_or_else(|| Error::Invalid("inline rule directory has no parent".to_owned()))?;
    let imported_parent = imported_rules
        .parent()
        .ok_or_else(|| Error::Invalid("imported rule directory has no parent".to_owned()))?;
    let inline_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
            &ryo_benchmark::outbound::context::factory::local(),
        )
        .prepare_cases(
            inline_parent,
            &root.join(format!("single-inline-cases-{rule}")),
        )?;
    let imported_cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite,
            &ryo_benchmark::outbound::context::factory::local(),
        )
        .prepare_cases(
            imported_parent,
            &root.join(format!("single-imported-cases-{rule}")),
        )?;
    let imported_by_name: BTreeMap<_, _> = imported_cases
        .into_iter()
        .map(|case| ((case.name.0.clone(), case.rule.as_ref().to_owned()), case))
        .collect();
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )]);
    let inline_engine = factory::local(languages.clone(), None);
    let imported_engine = factory::local(languages, None);
    let profile_root = imported_parent
        .parent()
        .ok_or_else(|| Error::Invalid("imported profile has no root".to_owned()))?;
    write_suppress_module(profile_root)?;
    for case in inline_cases {
        let key = (case.name.0.clone(), case.rule.as_ref().to_owned());
        let imported_case = imported_by_name
            .get(&key)
            .ok_or_else(|| Error::Invalid(format!("missing imported case for {key:?}")))?;
        let inline = inline_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&[rule])),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: case.request.root,
            rules: inline_parent.to_path_buf(),
            query_modules: BTreeMap::default(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            files: case.request.files,
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        let fragmented = imported_engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&[rule])),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
            root: imported_case.request.root.clone(),
            rules: imported_parent.to_path_buf(),
            query_modules: super::prepared_engine_test::module_names(suppress_modules()),
            rule_applicability: BTreeMap::default(),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
            files: imported_case.request.files.clone(),
            state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
        })?;
        assert_eq!(inline, fragmented, "single-rule mismatch for {key:?}");
    }
    Ok(())
}
